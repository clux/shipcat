use std::fs;
use std::fmt;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Write;

use serde_yaml;
use crate::webhooks::{self, UpgradeState};
use super::kube;
use super::Metadata;
use super::{Manifest, Config, Region};
use super::{Result, ResultExt, ErrorKind};
use super::helpers::{self, hout, hexec};

/// The different modes we allow `helm upgrade` to run in
#[derive(PartialEq, Clone, Debug)]
pub enum UpgradeMode {
    /// Upgrade dry-run
    DiffOnly,
    /// Normal Upgrade waiting for the calculated amount of time
    UpgradeWait,
    /// Upgrade, but don't wait
    UpgradeNoWait,
    /// Upgrade and wait, but also debug and rollback if helm returned 1
    UpgradeWaitMaybeRollback,
    /// Upgrade with force recreate pods
    UpgradeRecreateWait,
    /// Upgrade with install flag set
    UpgradeInstall,
    /// Upgrade or install, but always wait (reconcile)
    UpgradeInstallWait,
    /// Upgrade or install, but dont wait (apply with --no-wait)
    UpgradeInstallNoWait,
}
impl Default for UpgradeMode {
    fn default() -> Self {
        UpgradeMode::DiffOnly
    }
}

impl fmt::Display for UpgradeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            &UpgradeMode::DiffOnly => write!(f, "diff"),
            &UpgradeMode::UpgradeWait => write!(f, "upgrade"),
            &UpgradeMode::UpgradeNoWait => write!(f, "upgrade (fire and forget)"),
            &UpgradeMode::UpgradeRecreateWait => write!(f, "recreate"),
            &UpgradeMode::UpgradeInstall => write!(f, "install"),
            &UpgradeMode::UpgradeWaitMaybeRollback => write!(f, "upgrade"),
            &UpgradeMode::UpgradeInstallWait => write!(f, "reconcile"),
            &UpgradeMode::UpgradeInstallNoWait => write!(f, "reconciled (fire and forget)"),
        }
    }
}

impl UpgradeMode {
    pub fn action_verb(&self) -> String {
        match self {
            &UpgradeMode::DiffOnly => "diffed",
            &UpgradeMode::UpgradeWait => "upgraded",
            &UpgradeMode::UpgradeNoWait => "upgraded (fire and forget)",
            &UpgradeMode::UpgradeRecreateWait => "recreated pods for",
            &UpgradeMode::UpgradeInstall => "installed",
            &UpgradeMode::UpgradeWaitMaybeRollback => "upgraded",
            &UpgradeMode::UpgradeInstallWait => "reconciled",
            &UpgradeMode::UpgradeInstallNoWait => "reconciled (fire and forget)",
        }.into()
    }
}

/// Direct rollback command using synthetic or failed `UpgradeData`
///
/// Always just rolls back using to the helm's previous release and doesn't block
/// I.e. it assumes th previous release is stable and can be upgraded to.
/// If this is not the case you might have degraded service (fewer replicas).
///
/// TODO: deprecate
pub fn rollback(reg: &Region, ud: &UpgradeData, mf: &Manifest, conf: &Config) -> Result<()> {
    assert!(ud.namespace.len() > 0);
    let rollbackvec = vec![
        format!("--tiller-namespace={}", ud.namespace),
        "rollback".into(),
        ud.name.clone(),
        "0".into(), // magic helm number for previous
    ];
    info!("helm {}", rollbackvec.join(" "));

    webhooks::upgrade_rollback_event(UpgradeState::RollingBack, &ud, &reg, &conf);
    match hexec(rollbackvec) {
        Err(e) => {
            error!("{}", e);
            webhooks::upgrade_rollback_event(UpgradeState::RollbackFailed, &ud, &reg, &conf);
            Err(e)
        },
        Ok(_) => {
            let res = kube::await_rollout_status(&mf);
            webhooks::upgrade_rollback_event(UpgradeState::RolledBack, &ud, &reg, &conf);
            res?; // propagate errors from rollback check if any
            Ok(())
        }
    }
}

/// Rollback entrypoint using a plain service and region
pub fn rollback_wrapper(svc: &str, conf: &Config, region: &Region) -> Result<()> {
    let base = shipcat_filebacked::load_manifest(svc, &conf, region)?;
    let ud = UpgradeData::from_rollback(&base);
    rollback(&region, &ud, &base, &conf)
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
        // all entry points into this should set mf.version correctly - and call mf.verify
        let version = mf.version.clone().ok_or_else(|| ErrorKind::ManifestFailure("version".into()))?;

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
            waittime: mf.estimate_wait_time(),
            // empty diff
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

    // TODO: dedupe
    match data.mode {
        UpgradeMode::UpgradeWaitMaybeRollback |
        UpgradeMode::UpgradeWait |
        UpgradeMode::UpgradeNoWait => {},
        UpgradeMode::UpgradeRecreateWait => {
            upgradevec.extend_from_slice(&[
                "--recreate-pods".into(),
            ]);
        },
        UpgradeMode::UpgradeInstall |
        UpgradeMode::UpgradeInstallNoWait => {
            upgradevec.extend_from_slice(&[
                "--install".into(),
            ]);
        },
        UpgradeMode::UpgradeInstallWait => {
            upgradevec.extend_from_slice(&[
                "--install".into(),
            ]);
        },
        // TODO: handle apply correctly (depending on case)
        ref u => {
            unimplemented!("Somehow got an uncovered upgrade mode ({:?})", u);
        }
    }

    // CC service contacts on result
    info!("helm {}", upgradevec.join(" "));
    hexec(upgradevec).chain_err(||
        ErrorKind::HelmUpgradeFailure(data.name.clone())
    )
}

enum DiffMode {
    Upgrade,
    //Rollback,
}

impl fmt::Display for DiffMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
        mf.get_secrets()
    );
    if !hdifferr.is_empty() {
        if hdifferr.starts_with(&format!("Error: \"{}\" has no deployed releases", mf.name)) {
            let cmd = format!("helm --tiller-namespace={} del --purge {}", namespace, mf.name);
            let reason = "to let you be able to retry the install/reconcile";
            error!("Previous installs of {} failed, you need to run: \n\t{}\n{}",
                mf.name, cmd, reason
            );
            // TODO: automate above? feels dangerous..
            // return empty diff to force the error on helms end
            return Ok(format!("no deployed releases of {} - needs purge", mf.name));
        }
        warn!("diff {} stderr: \n{}", mf.name, hdifferr);
        if ! hdifferr.contains("error copying from local connection to remote stream") &&
           ! hdifferr.contains("error copying from remote stream to local connection") {
            bail!("diff plugin for {} returned: {}", mf.name, hdifferr.lines().next().unwrap());
        }
    }

    let smalldiff = helpers::diff_format(helmdiff.clone());

    if !helmdiff.is_empty() {
        debug!("{}", helmdiff); // full diff for logs
        println!("{}", smalldiff);
    } else {
        info!("{} is up to date", mf.name);
    }
    Ok(smalldiff)
}

/// Create helm values file for a service
///
/// Requires a completed manifest (with inlined configs)
pub fn values(mf: &Manifest, output: Option<String>) -> Result<()> {
    let encoded = serde_yaml::to_string(&mf)?;
    if let Some(o) = output {
        let pth = Path::new(".").join(o);
        debug!("Writing helm values for {} to {}", mf.name, pth.display());
        let mut f = File::create(&pth)?;
        writeln!(f, "{}", encoded)?;
        debug!("Wrote helm values for {} to {}: \n{}", mf.name, pth.display(), encoded);
    } else {
        println!("{}\n", encoded);
    }
    Ok(())
}

/// Analogue of helm template
///
/// Generates helm values to disk, then passes it to helm template
pub fn template(svc: &str, region: &Region, conf: &Config, ver: Option<String>, mock: bool, output: Option<PathBuf>) -> Result<String> {
    let mut mf = if mock {
        shipcat_filebacked::load_manifest(svc, conf, region)?.stub(region)?
    } else {
        shipcat_filebacked::load_manifest(svc, conf, region)?.complete(region)?
    };

    // template or values does not need version - but respect passed in / manifest
    if ver.is_some() {
        // override with set version only if set - respect pin otherwise
        mf.version = ver;
    }
    // sanity verify what we changed (no-shoehorning in illegal versions in rolling envs)
    if let Some(v) = &mf.version {
        region.versioningScheme.verify(&v)?;
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
    if let Some(o) = output {
        let pth = Path::new(".").join(o);
        info!("Writing helm template for {} to {}", svc, pth.display());
        let mut f = File::create(&pth)?;
        writeln!(f, "{}", tpl)?;
        debug!("Wrote helm template for {} to {}: \n{}", svc, pth.display(), tpl);
    } else {
        println!("{}", tpl);
    }
    fs::remove_file(hfile)?;
    Ok(tpl)
}

/// Helm history wrapper
///
/// Analogue to `helm history {service}` uses the right tiller namespace
pub fn history(svc: &str, conf: &Config, region: &Region) -> Result<()> {
    let mf = shipcat_filebacked::load_manifest(svc, &conf, region)?;
    let histvec = vec![
        format!("--tiller-namespace={}", mf.namespace),
        "history".into(),
        svc.into(),
    ];
    debug!("helm {}", histvec.join(" "));
    hexec(histvec)?;
    Ok(())
}

/// Helm status wrapper
///
/// Analogue to `helm status {service}` uses the right tiller namespace
pub fn status(svc: &str, conf: &Config, region: &Region) -> Result<()> {
    let mf = shipcat_filebacked::load_manifest(svc, &conf, region)?;
    let histvec = vec![
        format!("--tiller-namespace={}", mf.namespace),
        "status".into(),
        svc.into(),
    ];
    debug!("helm {}", histvec.join(" "));
    hexec(histvec)?;
    Ok(())
}

/// Handle error for a single upgrade
/// TODO: deprecate (see #183)
pub fn handle_upgrade_rollbacks(reg: &Region, u: &UpgradeData, mf: &Manifest, conf: &Config) -> Result<()> {
    match u.mode {
        UpgradeMode::UpgradeRecreateWait |
        UpgradeMode::UpgradeInstall |
        UpgradeMode::UpgradeWaitMaybeRollback => kube::debug(&mf)?,
        _ => {}
    }
    if u.mode == UpgradeMode::UpgradeWaitMaybeRollback {
        rollback(&reg, &u, mf, &conf)?;
    }
    Ok(())
}

/// Independent wrapper for helm values
///
/// Completes a manifest and prints it out with the given version
pub fn values_wrapper(svc: &str, region: &Region, conf: &Config, ver: Option<String>) -> Result<()> {
    let mut mf = shipcat_filebacked::load_manifest(svc, conf, region)?.complete(region)?;

    // template or values does not need version - but respect passed in / manifest
    if ver.is_some() {
        mf.version = ver;
    }
    // sanity verify what we changed (no-shoehorning in illegal versions in rolling envs)
    region.versioningScheme.verify(&mf.version.clone().unwrap())?;

    values(&mf, None)
}

/// Full helm wrapper for a single upgrade/diff/install
pub fn upgrade_wrapper(svc: &str, mode: UpgradeMode, region: &Region, conf: &Config, ver: Option<String>) -> Result<Option<UpgradeData>> {
    if let Err(e) = webhooks::ensure_requirements(&region) {
        warn!("Could not ensure webhook requirements: {}", e);
    }

    let mut mf = shipcat_filebacked::load_manifest(svc, conf, region)?.complete(region)?;

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
    let exists = mode != UpgradeMode::UpgradeInstall && mode != UpgradeMode::UpgradeInstallNoWait;
    // Other modes can infer in a pinch

    // ..but if they already exist on kube, don't block on that..
    if mf.version.is_none() {
        mf.version = Some(helpers::infer_fallback_version(&svc, &mf.namespace)?);
    };
    // sanity verify what we changed (no-shoehorning in illegal versions in rolling envs)
    region.versioningScheme.verify(&mf.version.clone().unwrap())?;

    // Template values file
    let hfile = format!("{}.helm.gen.yml", &svc);
    values(&mf, Some(hfile.clone()))?;

    // Sanity step that gives canonical upgrade data
    let upgrade_opt = UpgradeData::new(&mf, &hfile, mode, exists)?;
    if let Some(ref udata) = upgrade_opt {
        webhooks::upgrade_event(UpgradeState::Pending, &udata, &region, &conf);
        match upgrade(&udata) {
            Err(e) => {
                // if it failed here, rollback in job : TODO: FIX kube-deploy-X jobs
                error!("{} from {}", e, udata.name);
                // upgrade failed immediately - couldn't create resources
                webhooks::upgrade_event(UpgradeState::Failed, &udata, &region, &conf);
                handle_upgrade_rollbacks(&region, &udata, &mf, &conf)?; // for now leave it in..
                return Err(e);
            },
            Ok(_) => {
                // after helm upgrade / kubectl apply, check rollout status in a loop:
                if udata.mode == UpgradeMode::UpgradeNoWait || udata.mode == UpgradeMode::UpgradeInstallNoWait || kube::await_rollout_status(&mf)? {
                    info!("successfully rolled out {}", &udata.name);
                    webhooks::upgrade_event(UpgradeState::Completed, &udata, &region, &conf);
                }
                else {
                    let _ = kube::debug_rollout_status(&mf);
                    let _ = kube::debug(&mf);
                    warn!("failed to roll out {}", &udata.name);
                    webhooks::upgrade_event(UpgradeState::Failed, &udata, &region, &conf);
                    // if it failed here, rollback in job : TODO: FIX kube-deploy-X jobs
                    handle_upgrade_rollbacks(&region, &udata, &mf, &conf)?; // for now leave it in..
                    return Err(ErrorKind::UpgradeTimeout(mf.name.clone(), mf.estimate_wait_time()).into());
                }
            }
        };
    }

    let _ = fs::remove_file(&hfile); // try to remove temporary file
    Ok(upgrade_opt)
}
