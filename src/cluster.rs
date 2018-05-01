use super::helm::{self, UpgradeMode};
use super::{Result, Config, Manifest};

/// Helm upgrade the region (reconcile)
///
/// Upgrades multiple services at a time using rolling upgrade in a threadpool.
/// Ignores upgrade failures.
pub fn helm_reconcile(conf: &Config, region: String, n_workers: usize) -> Result<()> {
    mass_helm(conf, region, UpgradeMode::UpgradeWait, n_workers)
}

/// Helm installs region (disaster recovery)
///
/// Installs multiple services at a time in a threadpool.
/// This upgrade mode does not wait so this should only be limited by k8s.
pub fn helm_install(conf: &Config, region: String, n_workers: usize) -> Result<()> {
    mass_helm(conf, region, UpgradeMode::UpgradeInstall, n_workers)
}


/// Helm diff the region
///
/// Returns the diffs only from all services across a region.
/// Farms out the work to a thread pool.
pub fn helm_diff(conf: &Config, region: String, n_workers: usize) -> Result<()> {
    mass_helm(conf, region, UpgradeMode::DiffOnly, n_workers)
}


/// Experimental threaded mass helm operation
///
/// Reads secrets first, dumps all the helm values files,
/// then helm {operation} all the services.
/// This still might still use helm wait, but it does multiple services at a time.
fn mass_helm(conf: &Config, region: String, umode: UpgradeMode, n_workers: usize) -> Result<()> {
    let mut svcs = vec![];
    for svc in Manifest::available()? {
        debug!("Scanning service {:?}", svc);
        let mf = Manifest::basic(&svc, conf, None)?;
        if !mf.disabled && !mf.external && mf.regions.contains(&region) {
            svcs.push(svc);
        }
    }
    helm::parallel::upgrade(svcs, conf, region, umode, n_workers)
}
