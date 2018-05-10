use super::helm::{self, UpgradeMode};
use super::{Result, Config, Manifest};

/// Helm upgrade the region (reconcile)
///
/// Upgrades multiple services at a time using rolling upgrade in a threadpool.
/// Ignores upgrade failures.
pub fn helm_reconcile(conf: &Config, region: &str, n_workers: usize) -> Result<()> {
    mass_helm(conf, region, UpgradeMode::UpgradeWait, n_workers)
}

/// Helm installs region (disaster recovery)
///
/// Installs multiple services at a time in a threadpool.
/// This upgrade mode does not wait so this should only be limited by k8s.
pub fn helm_install(conf: &Config, region: &str, n_workers: usize) -> Result<()> {
    mass_helm(conf, region, UpgradeMode::UpgradeInstall, n_workers)
}

/// Helm diff the region
///
/// Returns the diffs only from all services across a region.
/// Farms out the work to a thread pool.
pub fn helm_diff(conf: &Config, region: &str, n_workers: usize) -> Result<()> {
    mass_helm(conf, region, UpgradeMode::DiffOnly, n_workers)
}

// Find all active services in a region and helm::parallel::upgrade them
fn mass_helm(conf: &Config, region: &str, umode: UpgradeMode, n_workers: usize) -> Result<()> {
    let mut svcs = vec![];
    for svc in Manifest::available()? {
        debug!("Scanning service {:?}", svc);
        let mf = Manifest::basic(&svc, conf, None)?;
        if !mf.disabled && !mf.external && mf.regions.contains(&region.to_string()) {
            svcs.push(mf);
        }
    }
    helm::parallel::reconcile(svcs, conf, region, umode, n_workers)
}
