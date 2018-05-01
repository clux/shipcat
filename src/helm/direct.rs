use std::fs;

use std::fmt;
use std::fs::File;
use std::path::{Path};
use std::io::{self, Write};

use super::slack;
use super::kube;
use super::generate::{self, Deployment};
use super::{Result, Manifest};
use super::helpers::{self, hout, hexec};

/// The different modes we allow `helm upgrade` to run in
#[derive(PartialEq, Clone)]
pub enum UpgradeMode {
    /// Upgrade dry-run
    DiffOnly,
    /// Normal Upgrade waiting for the calculated amount of time
    UpgradeWait,
    /// Upgrade and wait, but also debug and rollback if helm returned 1
    UpgradeWaitMaybeRollback,
    /// Upgrade with force recreate pods
    UpgradeRecreateWait,
    /// Upgrade with install flag set
    UpgradeInstall,
}
impl fmt::Display for UpgradeMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &UpgradeMode::DiffOnly => write!(f, "diff"),
            &UpgradeMode::UpgradeWait => write!(f, "blindly upgrade"),
            &UpgradeMode::UpgradeRecreateWait => write!(f, "recreate"),
            &UpgradeMode::UpgradeInstall => write!(f, "install"),
            &UpgradeMode::UpgradeWaitMaybeRollback => write!(f, "upgrade"),
        }
    }
}


// debugging when helm upgrade fails
fn kube_debug(mf: &Manifest) -> Result<()> {
    let pods = kube::get_broken_pods(&mf.name)?;
    for pod in pods.clone() {
        warn!("Debugging non-running pod {}", pod);
        warn!("Last 30 log lines:");
        let logvec = vec![
            "logs".into(),
            pod.clone(),
            format!("--tail=30").into(),
        ];
        match kube::kout(logvec) {
            Ok(l) => {
                // TODO: stderr?
                print!("{}\n", l);
            },
            Err(e) => {
                warn!("Failed to get logs from {}: {}", pod, e)
            }
        }
    }

    for pod in pods {
        warn!("Describing events for pod {}", pod);
        let descvec = vec![
            "describe".into(),
            "pod".into(),
            pod.clone()
        ];
        match kube::kout(descvec) {
            Ok(mut o) => {
                if let Some(idx) = o.find("Events:\n") {
                    print!("{}\n", o.split_off(idx))
                }
                else {
                    // Not printing in this case, tons of secrets in here
                    warn!("Unable to find events for pod {}", pod);
                }
            },
            Err(e) => {
                warn!("Failed to describe {}: {}", pod, e)
            }
        }
    }
    Ok(())
}

fn rollback(mf: &Manifest, namespace: &str) -> Result<()> {
    let rollbackvec = vec![
        format!("--tiller-namespace={}", namespace),
        "rollback".into(),
        mf.name.clone(),
        "0".into(), // magic helm string for previous
    ];
    // TODO: diff this rollback? https://github.com/databus23/helm-diff/issues/6
    info!("helm {}", rollbackvec.join(" "));
    match hexec(rollbackvec) {
        Err(e) => {
            error!("{}", e);
            // this would be super weird, since we are not waiting for it:
            let _ = slack::send(slack::Message {
                text: format!("failed to rollback `{}` in {}", &mf.name, &mf._region),
                color: Some("danger".into()),
                link: helpers::infer_ci_links(),
                ..Default::default()
            });
            Err(e)
        },
        Ok(_) => {
            slack::send(slack::Message {
                text: format!("rolling back `{}` in {}",  &mf.name, &mf._region),
                color: Some("good".into()),
                link: helpers::infer_ci_links(),
                ..Default::default()
            })?;
            Ok(())
        }
    }
}


/// Helm upgrade a service in one of various modes
///
/// This will use the explicit version set in the manifest (at this point).
/// It assumes that the helm values has been written to the `hfile`.
///
/// It then figures out the correct chart, and upgrade in the correct way.
/// See the `UpgradeMode` enum for more info.
/// In the `UpgradeWaitMaybeRollback` we also will roll back if helm upgrade failed,
/// but only after some base level debug has been output to console.
pub fn upgrade(mf: &Manifest, hfile: &str, mode: UpgradeMode) -> Result<(Manifest, String)> {
    if mode != UpgradeMode::DiffOnly {
        slack::have_credentials()?;
    }
    let namespace = kube::current_namespace(&mf._region)?;

    let helmdiff = if mode == UpgradeMode::UpgradeInstall {
        "".into() // can't diff against what's not there!
    } else {
        let hdiff = diff(mf, hfile)?;
        if mode == UpgradeMode::DiffOnly {
            return Ok((mf.clone(), hdiff))
        }
        hdiff
    };

    let ver = helpers::version_validate(&mf)?;


    if mode == UpgradeMode::UpgradeRecreateWait || mode == UpgradeMode::UpgradeInstall || !helmdiff.is_empty() {
        // upgrade it using the same command
        let mut upgradevec = vec![
            format!("--tiller-namespace={}", namespace),
            "upgrade".into(),
            mf.name.clone(),
            format!("charts/{}", mf.chart),
            "-f".into(),
            hfile.into(),
            "--set".into(),
            format!("version={}", ver),
        ];

        match mode {
            UpgradeMode::UpgradeWaitMaybeRollback | UpgradeMode::UpgradeWait => {
                upgradevec.extend_from_slice(&[
                    "--wait".into(),
                    format!("--timeout={}", helpers::calculate_wait_time(mf)),
                ]);
            },
            UpgradeMode::UpgradeRecreateWait => {
                upgradevec.extend_from_slice(&[
                    "--recreate-pods".into(),
                    "--wait".into(),
                    format!("--timeout={}", helpers::calculate_wait_time(mf)),
                ]);
            },
            UpgradeMode::UpgradeInstall => {
                upgradevec.extend_from_slice(&[
                    "--install".into(),
                ]);
            },
            _ => {
                unimplemented!("Somehow got an uncovered upgrade mode");
            }
        }

        // CC service contacts on result
        info!("helm {}", upgradevec.join(" "));
        let notifies = mf.metadata.clone().contacts;
        match hexec(upgradevec) {
            Err(e) => {
                error!("{} from {}", e, mf.name);
                slack::send(slack::Message {
                    text: format!("failed to {} `{}` in {}", mode, &mf.name, &mf._region),
                    color: Some("danger".into()),
                    link: helpers::infer_ci_links(),
                    notifies,
                    code: Some(helmdiff.clone()),
                })?;
                if mode == UpgradeMode::UpgradeWaitMaybeRollback {
                    kube_debug(mf)?;
                    rollback(mf, &namespace)?;
                }
                return Err(e);
            },
            Ok(_) => {
                // TODO: gh link!
                slack::send(slack::Message {
                    text: format!("{}d `{}` in {}", mode, &mf.name, &mf._region),
                    color: Some("good".into()),
                    notifies,
                    link: helpers::infer_ci_links(),
                    code: Some(helmdiff.clone()),
                })?;
            }
        };
    }
    Ok((mf.clone(), helmdiff))
}


/// helm diff against current running release
///
/// Shells out to helm diff, then obfuscates secrets
pub fn diff(mf: &Manifest, hfile: &str) -> Result<String> {
    let ver = mf.version.clone().unwrap(); // must be set outside
    let namespace = kube::current_namespace(&mf._region)?;
    let diffvec = vec![
        format!("--tiller-namespace={}", namespace),
        "diff".into(),
        "upgrade".into(),
        "--no-color".into(),
        "-q".into(),
        mf.name.clone(),
        format!("charts/{}", mf.chart),
        "-f".into(),
        hfile.into(),
        format!("--version={}", ver),
    ];
    info!("helm {}", diffvec.join(" "));
    let (hdiffunobfusc, hdifferr, _) = hout(diffvec.clone())?;
    let helmdiff = helpers::obfuscate_secrets(
        hdiffunobfusc,
        mf._decoded_secrets.values().cloned().collect()
    );
    if !hdifferr.is_empty() {
        warn!("diff {} stderr: \n{}", mf.name, hdifferr);
        if ! hdifferr.contains("error copying from local connection to remote stream") {
            bail!("diff plugin for {} returned: {}", mf.name, hdifferr.lines().next().unwrap());
        }
    }
    let smalldiff = helpers::diff_format(helmdiff.clone());

    if !helmdiff.is_empty() {
        debug!("{}\n", helmdiff); // full diff for logs
        print!("{}\n", smalldiff);
    } else {
        info!("{} is up to date", mf.name);
    }
    Ok(smalldiff)
}

/// Create helm values file for a service
///
/// Defers to `generate::helm` for now
pub fn values(dep: &Deployment, output: Option<String>, silent: bool) -> Result<Manifest> {
    generate::helm(dep, output, silent)
}


/// Analogue of helm template
///
/// Generates helm values to disk, then passes it to helm template
pub fn template(dep: &Deployment, output: Option<String>) -> Result<String> {
    let tmpfile = format!("{}.helm.gen.yml", dep.service);
    let _mf = generate::helm(dep, Some(tmpfile.clone()), true)?;

    // helm template with correct params
    let tplvec = vec![
        "template".into(),
        format!("charts/{}", dep.manifest.chart),
        "-f".into(),
        tmpfile.clone(),
    ];
    // NB: this call does NOT need --tiller-namespace (offline call)
    let (tpl, tplerr, success) = hout(tplvec.clone())?;
    if !success {
        warn!("{} stderr: {}", tplvec.join(" "), tplerr);
        bail!("helm template failed");
    }
    if let Some(o) = output {
        let pth = Path::new(".").join(o);
        info!("Writing helm template for {} to {}", dep.service, pth.display());
        let mut f = File::create(&pth)?;
        write!(f, "{}\n", tpl)?;
        debug!("Wrote helm template for {} to {}: \n{}", dep.service, pth.display(), tpl);
    } else {
        //stdout only
        let _ = io::stdout().write(tpl.as_bytes());
    }
    fs::remove_file(tmpfile)?;
    Ok(tpl)
}

/// Helm history wrapper
///
/// Analogue to `helm history {service}` uses the right tiller namespace
pub fn history(mf: &Manifest) -> Result<()> {
    let namespace = kube::current_namespace(&mf._region)?;
    let histvec = vec![
        format!("--tiller-namespace={}", namespace),
        "history".into(),
        mf.name.clone(),
    ];
    debug!("helm {}", histvec.join(" "));
    hexec(histvec)?;
    Ok(())
}
