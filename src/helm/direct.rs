use std::fs;
use std::fmt;
use std::io::{self, Write};

use super::slack;
use super::kube;
use super::Metadata;
use super::{Result, Manifest, ErrorKind, Error, Config};
use super::helpers::{self, hout, hexec};

/// The different modes we allow `helm upgrade` to run in
#[derive(PartialEq, Clone, Debug)]
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
    /// Upgrade or install, but always wait (reconcile)
    UpgradeInstallWait,
}
impl Default for UpgradeMode {
    fn default() -> Self {
        UpgradeMode::DiffOnly
    }
}

impl fmt::Display for UpgradeMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &UpgradeMode::DiffOnly => write!(f, "diff"),
            &UpgradeMode::UpgradeWait => write!(f, "blindly upgrade"),
            &UpgradeMode::UpgradeRecreateWait => write!(f, "recreate"),
            &UpgradeMode::UpgradeInstall => write!(f, "install"),
            &UpgradeMode::UpgradeWaitMaybeRollback => write!(f, "upgrade"),
            &UpgradeMode::UpgradeInstallWait => write!(f, "reconcile"),
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
            &UpgradeMode::UpgradeInstallWait => "reconciled",
        }.into()
    }
}

/// Direct rollback command using synthetic or failed `UpgradeData`
///
/// Always just rolls back using to the helm's previous release and doesn't block
/// I.e. it assumes th previous release is stable and can be upgraded to.
/// If this is not the case you might have degraded service (fewer replicas).
pub fn rollback(ud: &UpgradeData) -> Result<()> {
    assert!(ud.namespace.len() > 0);
    let rollbackvec = vec![
        format!("--tiller-namespace={}", ud.namespace),
        "rollback".into(),
        ud.name.clone(),
        "0".into(), // magic helm number for previous
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
                color: Some("warning".into()),
                metadata: ud.metadata.clone(),
                ..Default::default()
            })?;
            Ok(())
        }
    }
}

/// Rollback entrypoint using a plain service and region
pub fn rollback_wrapper(svc: &str, conf: &Config, region: &str) -> Result<()> {
    let base = Manifest::stubbed(svc, conf, region)?;
    let ud = UpgradeData::from_rollback(&base);
    rollback(&ud)
}

// All data needed for an upgrade
#[derive(Default, Clone)]
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

impl UpgradeData {
    /// Prepare an upgrade data from values and manifest data
    ///
    /// Manifest must have had a version set or inferred as appropriate.
    /// (DiffOnly can sneak by early due to it not being technically needed)
    ///
    /// Performs basic sanity checks, and populates canonical values that are reused a lot.
    pub fn new(mf: &Manifest, hfile: &str, mode: UpgradeMode, exists: bool) ->  Result<Option<UpgradeData>> {
        if mode != UpgradeMode::DiffOnly {
            slack::have_credentials()?;
        }
        let helmdiff = if !exists {
            "".into() // can't diff against what's not there!
        } else {
            let hdiff = diff(mf, hfile, DiffMode::Upgrade)?;
            if mode == UpgradeMode::DiffOnly {
                return Ok(None)
            }
            if hdiff.is_empty() && mode != UpgradeMode::UpgradeRecreateWait {
                debug!("Not upgrading {} - empty diff", mf.name);
                return Ok(None)
            }
            hdiff
        };

        // version + image MUST be set at this point before calling this for upgrade/install purposes
        // all entry points into this should set mf.version correctly!
        let version = mf.version.clone().ok_or_else(|| ErrorKind::ManifestFailure("version".into()))?;
        if let Err(e) = helpers::version_validate(&version) {
            warn!("Please supply a 40 char git sha version, or a semver version for {}", mf.name);
            //let img = mf.image.clone().ok_or_else(|| ErrorKind::ManifestFailure("image".into()))?;
            //if img.contains("quay.io/babylon") { // TODO: locked down repos in config
            return Err(e);
            //}
        }

        Ok(Some(UpgradeData {
            name: mf.name.clone(),
            diff: helmdiff,
            metadata: mf.metadata.clone(),
            chart: mf.chart.clone().unwrap(),
            waittime: mf.estimate_wait_time(),
            region: mf.region.clone(),
            values: hfile.into(),
            namespace: mf.namespace.clone(),
            mode, version
        }))
    }

    pub fn from_install(mf: &Manifest) -> UpgradeData {
        UpgradeData {
            name: mf.name.clone(),
            version: mf.version.clone().unwrap_or_else(|| "unknown".into()),
            metadata: mf.metadata.clone(),
            region: mf.region.clone(),
            chart: mf.chart.clone().unwrap(),
            mode: UpgradeMode::UpgradeInstall,
            // empty diff, namespace, 0 waittime,
            ..Default::default()
        }
    }
    pub fn from_rollback(mf: &Manifest) -> UpgradeData {
        UpgradeData {
            name: mf.name.clone(),
            version: "unset".into(),
            metadata: mf.metadata.clone(),
            namespace: mf.namespace.clone(),
            region: mf.region.clone(),
            chart: mf.chart.clone().unwrap(), // helm doesn't need this to rollback, but setting
            mode: UpgradeMode::UpgradeInstall, // unused in rollback flow, but setting
            // empty diff, 0 waittime,
            ..Default::default()
        }
    }
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
        UpgradeMode::UpgradeInstallWait => {
              upgradevec.extend_from_slice(&[
                "--install".into(),
                "--wait".into(),
                format!("--timeout={}", data.waittime),
            ]);
        },
        ref u => {
            unimplemented!("Somehow got an uncovered upgrade mode ({:?})", u);
        }
    }

    // CC service contacts on result
    info!("helm {}", upgradevec.join(" "));
    hexec(upgradevec).map_err(|e| ErrorKind::HelmUpgradeFailure(e.to_string()).into())
}

enum DiffMode {
    Upgrade,
    //Rollback,
}

impl fmt::Display for DiffMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &DiffMode::Upgrade => write!(f, "upgrade"),
            //&DiffMode::Rollback => write!(f, "rollback"),
        }
    }
}

/// helm diff against current running release
///
/// Shells out to helm diff, then obfuscates secrets
fn diff(mf: &Manifest, hfile: &str, dmode: DiffMode) -> Result<String> {
    let ver = mf.version.clone().unwrap(); // must be set outside
    let namespace = mf.namespace.clone();
    let diffvec = vec![
        format!("--tiller-namespace={}", namespace),
        "diff".into(),
        dmode.to_string(),
        "--no-color".into(),
        "-q".into(),
        mf.name.clone(),
        format!("charts/{}", mf.chart.clone().unwrap()),
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
        if ! hdifferr.contains("error copying from local connection to remote stream") &&
           ! hdifferr.contains("error copying from remote stream to local connection") {
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
    use std::io::prelude::*;
    use std::io;
    use serde_yaml;
    use std::path::Path;
    use std::fs::File;

    let encoded = serde_yaml::to_string(&mf)?;
    if let Some(o) = output {
        let pth = Path::new(".").join(o);
        debug!("Writing helm values for {} to {}", mf.name, pth.display());
        let mut f = File::create(&pth)?;
        write!(f, "{}\n", encoded)?;
        debug!("Wrote helm values for {} to {}: \n{}", mf.name, pth.display(), encoded);
    } else {
        let _ = io::stdout().write(format!("{}\n", encoded).as_bytes());
    }
    Ok(())
}

/// Analogue of helm template
///
/// Generates helm values to disk, then passes it to helm template
pub fn template(svc: &str, region: &str, conf: &Config, ver: Option<String>) -> Result<String> {
    let mut mf = Manifest::completed(svc, &conf, region)?;
    mf.verify(conf)?;

    // template or values does not need version - but respect passed in / manifest
    if ver.is_some() {
        // override with set version only if set - respect pin otherwise
        mf.version = ver;
    }

    let hfile = format!("{}.helm.gen.yml", svc);
    values(&mf, Some(hfile.clone()))?;

    // helm template with correct params
    let tplvec = vec![
        "template".into(),
        format!("charts/{}", mf.chart.unwrap()),
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
pub fn history(svc: &str, conf: &Config, region: &str) -> Result<()> {
    let mf = Manifest::stubbed(svc, &conf, region)?;
    let histvec = vec![
        format!("--tiller-namespace={}", mf.namespace),
        "history".into(),
        svc.into(),
    ];
    debug!("helm {}", histvec.join(" "));
    hexec(histvec)?;
    Ok(())
}


/// Handle error for a single upgrade
pub fn handle_upgrade_rollbacks(e: &Error, u: &UpgradeData, mf: &Manifest) -> Result<()> {
    error!("{} from {}", e, u.name);
    match u.mode {
        UpgradeMode::UpgradeRecreateWait |
        UpgradeMode::UpgradeInstall |
        UpgradeMode::UpgradeWaitMaybeRollback => kube::debug(&mf)?,
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
        ("good".into(), format!("{} `{}` in `{}`", u.mode.action_verb(), u.name, u.region))
    } else {
        ("danger".into(), format!("failed to {} `{}` in `{}`", u.mode, u.name, u.region))
    };
    let code = if u.diff.is_empty() { None } else { Some(u.diff.clone()) };
    slack::send(slack::Message {
        text, code,
        color: Some(color),
        version: Some(u.version.clone()),
        metadata: u.metadata.clone(),
        ..Default::default()
    })
}

/// Independent wrapper for helm values
///
/// Completes a manifest and prints it out with the given version
pub fn values_wrapper(svc: &str, region: &str, conf: &Config, ver: Option<String>) -> Result<()> {
    let mut mf = Manifest::completed(svc, &conf, region)?;
    mf.verify(conf)?;
    // template or values does not need version - but respect passed in / manifest
    if ver.is_some() {
        mf.version = ver;
    }
    values(&mf, None)
}

/// Full helm wrapper for a single upgrade/diff/install
pub fn upgrade_wrapper(svc: &str, mode: UpgradeMode, region: &str, conf: &Config, ver: Option<String>) -> Result<Option<UpgradeData>> {
    let mut mf = Manifest::completed(svc, &conf, region)?;
    mf.verify(conf)?;

    // Ensure we have a version - or are able to infer one
    if ver.is_some() {
        mf.version = ver; // override if passing in
    }
    // Can't install without a version
    if mf.version.is_none() && mode == UpgradeMode::UpgradeInstall {
        warn!("No version found in either manifest or passed explicitly");
        bail!("helm install needs an explicit version")
    }
    // assume it exists if we're not doing installs
    // (this is fine atm because upgrade_wrapper is the CLI entrypoint)
    let exists = mode != UpgradeMode::UpgradeInstall;
    // Other modes can infer in a pinch

    if mf.version.is_none() {
        mf.version = Some(helpers::infer_fallback_version(&svc, &mf.namespace)?);
    };
    if mode != UpgradeMode::DiffOnly {
        // validate that the version matches the versioning scheme for this region
        // NB: only doing this on direct upgrades not parallel reconciles atm
        helpers::version_validate_specific(
            &mf.version.clone().unwrap(),
            &conf.regions[region].versioningScheme
        )?;
    }

    // Template values file
    let hfile = format!("{}.helm.gen.yml", &svc);
    values(&mf, Some(hfile.clone()))?;

    // Sanity step that gives canonical upgrade data
    let upgrade_opt = UpgradeData::new(&mf, &hfile, mode, exists)?;
    if let Some(ref udata) = upgrade_opt {
        // Upgrade necessary - pass on data:
        match upgrade(&udata) {
            Err(e) => {
                handle_upgrade_notifies(false, &udata)?;
                // TODO: kube debug doesn't seem to hit?
                handle_upgrade_rollbacks(&e, &udata, &mf)?;
                return Err(e);
            },
            Ok(_) => {
                handle_upgrade_notifies(true, &udata)?
            }
        };
    }
    let _ = fs::remove_file(&hfile); // try to remove temporary file
    Ok(upgrade_opt)
}


#[cfg(test)]
mod tests {
    use super::super::Manifest;
    use super::values;
    use tests::setup;
    use super::super::Config;

    #[test]
    fn helm_values() {
        setup();
        let conf = Config::read().unwrap();
        let mf = Manifest::stubbed("fake-ask", &conf, "dev-uk".into()).unwrap();
        if let Err(e) = values(&mf, None) {
            println!("Failed to create helm values for fake-ask");
            print!("{}", e);
            assert!(false);
        }
        // can verify output here matches what we want if we wanted to,
        // but type safety proves 99% of that anyway
    }
}
