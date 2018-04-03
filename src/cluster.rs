use threadpool::ThreadPool;
use std::sync::mpsc::channel;

use super::vault;
use super::generate::Deployment;
use super::template;
use super::helm::{self, UpgradeMode};
use super::{Result, Config, Manifest};



/// Helm upgrade the region (reconcile)
///
/// Upgrades multiple services at a time using rolling upgrade in a threadpool.
/// Ignores upgrade failures.
pub fn helm_reconcile(conf: &Config, region: String) -> Result<()> {
    mass_helm(conf, region, UpgradeMode::UpgradeWait)
}

/// Helm installs region (disaster recovery)
///
/// Installs multiple services at a time in a threadpool.
/// This upgrade mode does not wait so this should only be limited by k8s.
pub fn helm_install(conf: &Config, region: String) -> Result<()> {
    mass_helm(conf, region, UpgradeMode::UpgradeInstall)
}


/// Helm diff the region
///
/// Returns the diffs only from all services across a region.
/// Farms out the work to a thread pool.
pub fn helm_diff(conf: &Config, region: String) -> Result<()> {
    mass_helm(conf, region, UpgradeMode::DiffOnly)
}


/// Experimental threaded mass helm operation
///
/// Reads secrets first, dumps all the helm values files,
/// then helm {operation} all the services.
/// This still might still use helm wait, but it does multiple services at a time.
fn mass_helm(conf: &Config, region: String, umode: UpgradeMode) -> Result<()> {
    let services = Manifest::available()?;
    let mut manifests = vec![];
    for svc in services {
        debug!("Scanning service {:?}", svc);
        let mf = Manifest::basic(&svc, conf, None)?;
        if !mf.disabled && !mf.external && mf.regions.contains(&region) {
            // need a tera per service (special folder handling)
            let tera = template::init(&svc)?;
            let v = vault::Vault::default()?;
            let mut compmf = Manifest::completed(&region, &conf, &svc, Some(v))?;
            let regdefaults = conf.regions.get(&region).unwrap().defaults.clone();
            compmf.version = Some(helm::infer_version(&svc, &regdefaults)?);
            let dep = Deployment {
                service: svc.into(),
                region: region.clone(),
                manifest: compmf,
                render: Box::new(move |tmpl, context| {
                    template::render(&tera, tmpl, context)
                }),
            };
            // create all the values first
            let hfile = format!("{}.helm.gen.yml", dep.service);
            let mfrender = helm::values(&dep, Some(hfile.clone()), false)?;
            manifests.push(mfrender);
        }
    }

    let n_workers = 8;
    let n_jobs = manifests.len();
    let pool = ThreadPool::new(n_workers);
    info!("Reconciling {} jobs using {} workers", n_jobs, n_workers);

    let (tx, rx) = channel();
    for mf in manifests {
        let mode = umode.clone();
        let tx = tx.clone();
        pool.execute(move|| {
            let hfile = format!("{}.helm.gen.yml", mf.name); // as above
            let res = helm::upgrade(&mf, &hfile, mode);
            tx.send(res).expect("channel will be there waiting for the pool");
        });
    }

    // wait for threads and look for errors
    let mut res = rx.iter().take(n_jobs).map(|r| {
        match &r {
            &Ok((ref mf, _)) => debug!("{} {}", umode, mf.name),
            &Err(ref e) => error!("Failed to {} {}", umode, e)
        }
        r
    }).filter_map(Result::err);

    // propagate first error if exists
    if let Some(e) = res.next() {
        Err(e)
    } else {
        Ok(())
    }
}
