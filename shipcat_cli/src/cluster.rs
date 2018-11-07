use super::{Config, Backend, Region};
use super::helm::{self, UpgradeMode};
use super::{Result, Manifest};

/// Helm upgrade the region (reconcile)
///
/// Upgrades multiple services at a time using rolling upgrade in a threadpool.
/// Ignores upgrade failures.
pub fn helm_reconcile(conf: &Config, region: &Region, n_workers: usize) -> Result<()> {
    mass_helm(conf, region, UpgradeMode::UpgradeInstallWait, n_workers)
}

/// Helm diff the region
///
/// Returns the diffs only from all services across a region.
/// Farms out the work to a thread pool.
pub fn helm_diff(conf: &Config, region: &Region, n_workers: usize) -> Result<()> {
    mass_helm(conf, region, UpgradeMode::DiffOnly, n_workers)
}

// Find all active services in a region and helm::parallel::upgrade them
fn mass_helm(conf: &Config, region: &Region, umode: UpgradeMode, n_workers: usize) -> Result<()> {
    let mut svcs = vec![];
    for svc in Manifest::available(&region.name)? {
        debug!("Scanning service {:?}", svc);
        svcs.push(Manifest::base(&svc, conf, region)?);
    }
    helm::parallel::reconcile(svcs, conf, region, umode, n_workers)
}
