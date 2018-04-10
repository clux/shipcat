use std::fs;
use serde_yaml;

use std::fmt;
use std::fs::File;
use std::path::{Path};
use std::io::{self, Write};

use super::slack;
use super::kube;
use super::generate::{self, Deployment};
use super::{Result, Manifest};
use super::config::{RegionDefaults};

/// Values parsed from `helm get values {service}`
///
/// This is the completed manifests including templates, but we only need one key
/// Just parsing this key also makes it more forwards compatible
#[derive(Deserialize)]
struct HelmVals {
    version: String,
}

pub fn hexec(args: Vec<String>) -> Result<()> {
    use std::process::Command;
    debug!("helm {}", args.join(" "));
    let s = Command::new("helm").args(&args).status()?;
    if !s.success() {
        bail!("Subprocess failure from helm: {}", s.code().unwrap_or(1001))
    }
    Ok(())
}
pub fn hout(args: Vec<String>) -> Result<(String, String, bool)> {
    use std::process::Command;
    debug!("helm {}", args.join(" "));
    let s = Command::new("helm").args(&args).output()?;
    let out : String = String::from_utf8_lossy(&s.stdout).into();
    let err : String = String::from_utf8_lossy(&s.stderr).into();
    Ok((out, err, s.status.success()))
}


pub fn infer_version(service: &str, reg: &RegionDefaults) -> Result<String> {
    // fetch current version from helm
    let imgvec = vec![
        "get".into(),
        "values".into(),
        service.into(),
    ];
    debug!("helm {}", imgvec.join(" "));
    match hout(imgvec.clone()) {
        // got a result from helm + rc was 0:
        Ok((vout, verr, true)) => {
            if !verr.is_empty() {
                warn!("{} stderr: {}", imgvec.join(" "), verr);
            }
            // if we got this far, release was found
            // it should work to parse the HelmVals subset of the values:
            let values : HelmVals = serde_yaml::from_str(&vout.to_owned())?;
            Ok(values.version)
        },
        _ => {
            // nothing from helm, fallback to region defaults from config
            Ok(reg.version.clone())
        }
    }
}

fn infer_jenkins_link() -> Option<String> {
    use std::env;
    use std::process::Command;
    if let (Ok(url), Ok(name), Ok(nr)) = (env::var("BUILD_URL"),
                                          env::var("JOB_NAME"),
                                          env::var("BUILD_NUMBER")) {
        Some(format!("{}|{} #{}", url, name, nr))
    } else {
        match Command::new("whoami").output() {
            Ok(s) => {
                let mut out : String = String::from_utf8_lossy(&s.stdout).into();
                let len = out.len();
                if out.ends_with('\n') {
                    out.truncate(len - 1)
                }
                return Some(out)
            }
            Err(e) => {
                warn!("Could not retrieve user from shell {}", e);
                return None
            }
        }
    }
}

fn pre_upgrade_sanity() -> Result<()> {
    // TODO: kubectl auth can-i rollout Deployment

    // slack stuff must also be set:
    slack::env_channel()?;
    slack::env_hook_url()?;

    Ok(())
}

fn diff_format(diff: String) -> String {
    use regex::Regex;

    let diff_re = Regex::new(r"has changed|^\-|^\+").unwrap();
    // filter out lines that doesn't contain "has changed" or starting with + or -
    diff.split("\n").filter(|l| {
        diff_re.is_match(l)
    }).collect::<Vec<_>>().join("\n")
}

fn obfuscate_secrets(input: String, secrets: Vec<String>) -> String {
    let mut out = input.clone();
    for s in secrets {
        // If your secret is less than 8 characters, we won't obfuscate it
        // Mostly for fear of clashing with other parts of the output,
        // but also because it's an insecure secret anyway
        if s.len() >= 8 {
            out = out.replace(&s, "************");
        }
    }
    out
}

fn helm_wait_time(mf: &Manifest) -> u32 {
    // TODO: need to take into account image size!
    let rcount = mf.replicaCount.unwrap(); // this is set by defaults!
    if let Some(ref hc) = mf.health {
        // wait for at most 2 * bootTime * replicas
        2 * hc.wait * rcount
    } else {
        // sensible guess for boot time
        2 * 30 * rcount
    }
}

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
                    warn!("Unable to find events, describing pod:");
                    print!("{}\n", o);
                }
            },
            Err(e) => {
                warn!("Failed to desccribe {}: {}", pod, e)
            }
        }
    }
    Ok(())
}

fn rollback(mf: &Manifest) -> Result<()> {
    let rollbackvec = vec![
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
                text: format!("failed to rollback {} in {}", &mf.name, &mf._region),
                color: Some("danger".into()),
                link: infer_jenkins_link(),
                ..Default::default()
            });
            Err(e)
        },
        Ok(_) => {
            slack::send(slack::Message {
                text: format!("rolling back {} in {}",  &mf.name, &mf._region),
                color: Some("good".into()),
                link: infer_jenkins_link(),
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
        pre_upgrade_sanity()?;
    }
    let helmdiff = diff(mf, hfile)?;
    if mode == UpgradeMode::DiffOnly {
        return Ok((mf.clone(), helmdiff))
    }

    let ver = mf.version.clone().unwrap(); // must be set outside

    if mode == UpgradeMode::UpgradeRecreateWait || mode == UpgradeMode::UpgradeInstall || !helmdiff.is_empty() {
        // upgrade it using the same command
        let mut upgradevec = vec![
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
                    format!("--timeout={}", helm_wait_time(mf)),
                ]);
            },
            UpgradeMode::UpgradeRecreateWait => {
                upgradevec.extend_from_slice(&[
                    "--recreate-pods".into(),
                    "--wait".into(),
                    format!("--timeout={}", helm_wait_time(mf)),
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
                    text: format!("failed to {} {} in {}", mode, &mf.name, &mf._region),
                    color: Some("danger".into()),
                    link: infer_jenkins_link(),
                    notifies,
                    code: Some(helmdiff.clone()),
                })?;
                if mode == UpgradeMode::UpgradeWaitMaybeRollback {
                    kube_debug(mf)?;
                    rollback(mf)?;
                }
                return Err(e);
            },
            Ok(_) => {
                slack::send(slack::Message {
                    text: format!("{}d {} in {}", mode, &mf.name, &mf._region),
                    color: Some("good".into()),
                    notifies,
                    link: infer_jenkins_link(),
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
    let diffvec = vec![
        "diff".into(),
        "--no-color".into(),
        mf.name.clone(),
        format!("charts/{}", mf.chart),
        "-f".into(),
        hfile.into(),
        "--set".into(),
        format!("version={}", ver),
    ];
    info!("helm {}", diffvec.join(" "));
    let (hdiffunobfusc, hdifferr, _) = hout(diffvec.clone())?;
    let helmdiff = obfuscate_secrets(
        hdiffunobfusc,
        mf._decoded_secrets.values().cloned().collect()
    );
    if !hdifferr.is_empty() {
        warn!("diff {} stderr: \n{}", mf.name, hdifferr);
        bail!("diff plugin for {} returned: {}", mf.name, hdifferr.lines().next().unwrap());
    }
    let smalldiff = diff_format(helmdiff.clone());

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
