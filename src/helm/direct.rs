use std::fs;
use std::fmt;
use std::io::{self, Write};

use super::slack;
use super::kube;
use super::generate;
use super::Metadata;
use super::{Result, Manifest, ErrorKind, Error, Config};
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

impl UpgradeMode {
    fn action_verb(&self) -> String {
        match self {
            &UpgradeMode::DiffOnly => "diffed",
            &UpgradeMode::UpgradeWait => "blindly upgraded",
            &UpgradeMode::UpgradeRecreateWait => "recreated pods for",
            &UpgradeMode::UpgradeInstall => "installed",
            &UpgradeMode::UpgradeWaitMaybeRollback => "upgraded",
        }.into()
    }
}

// debugging when helm upgrade fails
fn kube_debug(svc: &str) -> Result<()> {
    let pods = kube::get_broken_pods(&svc)?;
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

fn rollback(ud: &UpgradeData) -> Result<()> {
    let rollbackvec = vec![
        format!("--tiller-namespace={}", ud.namespace),
        "rollback".into(),
        ud.name.clone(),
        "0".into(), // magic helm string for previous
    ];
    // TODO: diff this rollback? https://github.com/databus23/helm-diff/issues/6
    info!("helm {}", rollbackvec.join(" "));
    match hexec(rollbackvec) {
        Err(e) => {
            error!("{}", e);
            // this would be super weird, since we are not waiting for it:
            let _ = slack::send(slack::Message {
                text: format!("failed to rollback `{}` in {}", &ud.name, &ud.region),
                color: Some("danger".into()),
                metadata: ud.metadata.clone(),
                ..Default::default()
            });
            Err(e)
        },
        Ok(_) => {
            slack::send(slack::Message {
                text: format!("rolling back `{}` in {}",  &ud.name, &ud.region),
                color: Some("good".into()),
                metadata: ud.metadata.clone(),
                ..Default::default()
            })?;
            Ok(())
        }
    }
}

// All data needed for an upgrade
pub struct UpgradeData {
    /// Name of service
    pub name: String,
    /// Chart the service is using
    pub chart: String,
    /// Validated version string
    pub version: String,
    /// Validated region requested for installation
    pub region: String,
    /// Validated namespace inferred from region
    pub namespace: String,
    /// How long we will let helm wait before upgrading
    pub waittime: u32,
    /// Precomputed diff via helm diff plugin
    pub diff: String,
    /// Upgrade Mode
    pub mode: UpgradeMode,
    /// Path to helm values file
    pub values: String,
    /// Metadata used in slack notifications
    pub metadata: Option<Metadata>,
}

// Helm upgrade a service in one of various modes
//
// This will use the explicit version set in the manifest (at this point).
// It assumes that the helm values has been written to the `hfile`.
//
// It then figures out the correct chart, and upgrade in the correct way.
// See the `UpgradeMode` enum for more info.
// In the `UpgradeWaitMaybeRollback` we also will roll back if helm upgrade failed,
// but only after some base level debug has been output to console.
pub fn upgrade_wrapper(data: &UpgradeData) -> Result<()> {
    if data.mode == UpgradeMode::UpgradeRecreateWait ||
       data.mode == UpgradeMode::UpgradeInstall ||
       !data.diff.is_empty()
    {
        // We actually need to do something
        let res = upgrade(&data).map_err(|e| handle_upgrade_rollbacks(&e, &data));
        handle_upgrade_notifies(res.is_ok(), &data)?;
    }
    Ok(())
}

/// Prepare an upgrade data from values and manifest data
///
/// Performs basic sanity checks, and populates canonical values that are reused a lot.
pub fn upgrade_prepare(mf: &Manifest, hfile: &str, mode: UpgradeMode) -> Result<Option<UpgradeData>> {
    if mode != UpgradeMode::DiffOnly {
        slack::have_credentials()?;
    }
    let namespace = kube::current_namespace(&mf._region)?;

    let helmdiff = if mode == UpgradeMode::UpgradeInstall {
        "".into() // can't diff against what's not there!
    } else {
        let hdiff = diff(mf, hfile)?;
        if mode == UpgradeMode::DiffOnly {
            return Ok(None)
        }
        hdiff
    };

    // version + image MUST be set at this point before calling this
    let version = mf.version.clone().ok_or_else(|| ErrorKind::ManifestFailure("version".into()))?;
    if let Err(e) = helpers::version_validate(&version) {
        warn!("Please supply a 40 char git sha version, or a semver version for {}", mf.name);
        let img = mf.image.clone().ok_or_else(|| ErrorKind::ManifestFailure("image".into()))?;
        if img.contains("quay.io/babylon") { // TODO: locked down repos in config
            return Err(e);
        }
    }

    Ok(Some(UpgradeData {
        name: mf.name.clone(),
        diff: helmdiff,
        metadata: mf.metadata.clone(),
        chart: mf.chart.clone(),
        waittime: helpers::calculate_wait_time(mf),
        region: mf._region.clone(),
        values: hfile.into(),
        mode, version, namespace
    }))
}


pub fn upgrade(data: &UpgradeData) -> Result<()> {
    // upgrade it using the same command
    let mut upgradevec = vec![
        format!("--tiller-namespace={}", data.namespace),
        "upgrade".into(),
        data.name.clone(),
        format!("charts/{}", data.chart),
        "-f".into(),
        data.values.clone(),
        "--set".into(),
        format!("version={}", data.version),
    ];

    match data.mode {
        UpgradeMode::UpgradeWaitMaybeRollback | UpgradeMode::UpgradeWait => {
            upgradevec.extend_from_slice(&[
                "--wait".into(),
                format!("--timeout={}", data.waittime),
            ]);
        },
        UpgradeMode::UpgradeRecreateWait => {
            upgradevec.extend_from_slice(&[
                "--recreate-pods".into(),
                "--wait".into(),
                format!("--timeout={}", data.waittime),
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
    hexec(upgradevec)
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
/// Requires a completed manifest (with inlined configs)
pub fn values(mf: &Manifest, output: Option<String>) -> Result<()> {
    if let Some(o) = output {
        generate::values_to_disk(mf, &o)
    } else {
        generate::values_stdout(mf)
    }
}


/// Analogue of helm template
///
/// Generates helm values to disk, then passes it to helm template
pub fn template(svc: &str, region: &str, conf: &Config, ver: Option<String>) -> Result<String> {
    use super::vault;
    // Create a vault client and fetch all secrets
    let v = vault::Vault::default()?;
    let mut mf = Manifest::completed(svc, &conf, region, Some(v))?;

    // template or values does not need version - but respect passed in / manifest
    mf.version = if let Some(v) = ver {
        // If version set explicitly, use that
        Some(v)
    } else if let Some(v) = mf.version {
        // If pinned in manifests, respect that version
        Some(v)
    } else {
        // Should not need further network here..
        None
    };

    let hfile = format!("{}.helm.gen.yml", svc);
    values(&mf, Some(hfile.clone()))?;

    // helm template with correct params
    let tplvec = vec![
        "template".into(),
        format!("charts/{}", mf.chart),
        "-f".into(),
        hfile.clone(),
    ];
    // NB: this call does NOT need --tiller-namespace (offline call)
    let (tpl, tplerr, success) = hout(tplvec.clone())?;
    if !success {
        warn!("{} stderr: {}", tplvec.join(" "), tplerr);
        bail!("helm template failed");
    }
    //if let Some(o) = output {
    //    let pth = Path::new(".").join(o);
    //    info!("Writing helm template for {} to {}", dep.service, pth.display());
    //    let mut f = File::create(&pth)?;
    //    write!(f, "{}\n", tpl)?;
    //    debug!("Wrote helm template for {} to {}: \n{}", dep.service, pth.display(), tpl);
    //} else {
        //stdout only
        let _ = io::stdout().write(tpl.as_bytes());
    //}
    fs::remove_file(hfile)?;
    Ok(tpl)
}

/// Helm history wrapper
///
/// Analogue to `helm history {service}` uses the right tiller namespace
pub fn history(svc: &str, region: &str) -> Result<()> {
    let namespace = kube::current_namespace(region)?;
    let histvec = vec![
        format!("--tiller-namespace={}", namespace),
        "history".into(),
        svc.into(),
    ];
    debug!("helm {}", histvec.join(" "));
    hexec(histvec)?;
    Ok(())
}


/// Handle error for a single upgrade
pub fn handle_upgrade_rollbacks(e: &Error, u: &UpgradeData) -> Result<()> {
    error!("{} from {}", e, u.name);
    match u.mode {
        UpgradeMode::UpgradeRecreateWait |
        UpgradeMode::UpgradeInstall |
        UpgradeMode::UpgradeWaitMaybeRollback => kube_debug(&u.name)?,
        _ => {}
    }
    if u.mode == UpgradeMode::UpgradeWaitMaybeRollback {
        rollback(&u)?;
    }
    Ok(())
}

/// Notify slack upgrades from a single upgrade
pub fn handle_upgrade_notifies(success: bool, u: &UpgradeData) -> Result<()> {
    let (color, text) = if success {
        ("good".into(), format!("{} `{}`", u.mode.action_verb(), u.name))
    } else {
        ("danger".into(), format!("failed to {} `{}`", u.mode, u.name))
    };
    slack::send(slack::Message {
        text: text,
        color: Some(color),
        metadata: u.metadata.clone(),
        code: Some(u.diff.clone()),
        ..Default::default()
    })
}

/// Independent wrapper for helm values
///
/// Completes a manifest and prints it out with the given version
pub fn values_wrapper(svc: &str, region: &str, conf: &Config, ver: Option<String>) -> Result<()> {
    use super::vault;

    // Create a vault client and fetch all secrets
    let v = vault::Vault::default()?;
    let mut mf = Manifest::completed(svc, &conf, region, Some(v))?;

    // template or values does not need version - but respect passed in / manifest
    mf.version = if let Some(v) = ver {
        // If version set explicitly, use that
        Some(v)
    } else if let Some(v) = mf.version {
        // If pinned in manifests, respect that version
        Some(v)
    } else {
        // Should not need further network here..
        None
    };
    assert!(mf.regions.contains(&region.to_string()));
    values(&mf, None)
}

/// Full helm wrapper for a single upgrade/diff/install
pub fn full_wrapper(svc: &str, mode: UpgradeMode, region: &str, conf: &Config, ver: Option<String>) -> Result<Option<UpgradeData>> {
    use super::vault;

    // Create a vault client and fetch all secrets
    let v = vault::Vault::default()?;
    let mut mf = Manifest::completed(svc, &conf, region, Some(v))?;

    // Ensure we have a version - or are able to infer one
    mf.version = if let Some(v) = ver {
        // If version set explicitly, use that
        Some(v)
    } else if let Some(v) = mf.version {
        // If pinned in manifests, respect that version
        Some(v)
    } else if mode == UpgradeMode::UpgradeInstall {
        warn!("No version found in either manifest or passed explicitly");
        bail!("helm install needs an explicit version")
    } else {
        // Environment uses rolling upgrades - infer from current running
        let regdefaults = if let Some(r) = conf.regions.get(region) {
            r.defaults.clone()
        } else {
            bail!("You need to define the kube context '{}' in shipcat.conf fist", region)
        };
        Some(helpers::infer_fallback_version(&svc, &regdefaults)?)
    };

    // Template values file
    let hfile = format!("{}.helm.gen.yml", &svc);
    values(&mf, Some(hfile.clone()))?;

    // Sanity step that gives canonical upgrade data
    let upgrade_opt = upgrade_prepare(&mf, &hfile, mode)?;
    if let Some(ref udata) = upgrade_opt {
        // Upgrade necessary - pass on data:
        let res = upgrade(&udata).map_err(|e| {
            handle_upgrade_rollbacks(&e, &udata)
        });
        // notify about the result directly as they happen
        handle_upgrade_notifies(res.is_ok(), &udata)?;
    }
    let _ = fs::remove_file(&hfile); // try to remove temporary file
    Ok(upgrade_opt)
}
