use super::{Config, Region};
use super::helm::{self, UpgradeMode};
use super::{Result, Manifest};
use crate::webhooks;

/// Helm upgrade the region (reconcile)
///
/// Upgrades multiple services at a time using rolling upgrade in a threadpool.
/// Ignores upgrade failures.
pub fn helm_reconcile(conf: &Config, region: &Region, n_workers: usize) -> Result<()> {
    if let Err(e) = webhooks::ensure_requirements(&region) {
        warn!("Could not ensure webhook requirements: {}", e);
    }
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


/// Apply all crds in a region
///
/// Temporary helper that shells out to kubectl apply in parallel.
/// This will go away with catapult.
pub fn mass_crd(conf: &Config, reg: &Region, n_workers: usize) -> Result<()> {
    crd_reconcile(Manifest::available(&reg.name)?, conf, reg, n_workers)
}

use super::kube;
fn crd_reconcile(svcs: Vec<String>, config: &Config, region: &Region, n_workers: usize) -> Result<()> {
    use threadpool::ThreadPool;
    use std::sync::mpsc::channel;

    // Make sure config can apply first
    kube::apply_crd(&region.name, config.clone(), &region.namespace)?;

    // Single instruction kubectl delete shipcat manifests .... of excess ones
    kube::remove_redundant_manifests(&region.namespace, &svcs)?;

    let n_jobs = svcs.len();
    let pool = ThreadPool::new(n_workers);
    info!("Starting {} parallel kube jobs using {} workers", n_jobs, n_workers);

    // then parallel apply the remaining ones
    let (tx, rx) = channel();
    for svc in svcs {
        let reg = region.clone();
        let conf = config.clone();

        let tx = tx.clone(); // tx channel reused in each thread
        pool.execute(move || {
            debug!("Running CRD reconcile for {}", svc);
            let res = crd_reconcile_worker(svc, conf, reg);
            tx.send(res).expect("channel will be there waiting for the pool");
        });
    }
    // wait for threads collect errors
    let res = rx.iter().take(n_jobs).map(|r| {
        match &r {
            &Ok(_) => {},
            &Err(ref e) => warn!("error: {}", e),
        }
        r
    }).filter_map(Result::err).collect::<Vec<_>>();
    // propagate first non-ignorable error if exists
    if let Some(e) = res.into_iter().next() {
        // no errors ignoreable atm
        return Err(e)
    }
    Ok(())
}

fn crd_reconcile_worker(svc: String, conf: Config, reg: Region) -> Result<()> {
    let mf = Manifest::base(&svc, &conf, &reg)?;
    kube::apply_crd(&svc, mf, &reg.namespace)?;
    Ok(())
}
