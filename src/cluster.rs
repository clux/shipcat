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
pub fn helm_reconcile(conf: &Config, region: String, numjobs: usize) -> Result<()> {
    mass_helm(conf, region, UpgradeMode::UpgradeWait, numjobs)
}

/// Helm installs region (disaster recovery)
///
/// Installs multiple services at a time in a threadpool.
/// This upgrade mode does not wait so this should only be limited by k8s.
pub fn helm_install(conf: &Config, region: String, numjobs: usize) -> Result<()> {
    mass_helm(conf, region, UpgradeMode::UpgradeInstall, numjobs)
}


/// Helm diff the region
///
/// Returns the diffs only from all services across a region.
/// Farms out the work to a thread pool.
pub fn helm_diff(conf: &Config, region: String, numjobs: usize) -> Result<()> {
    mass_helm(conf, region, UpgradeMode::DiffOnly, numjobs)
}


/// Experimental threaded mass helm operation
///
/// Reads secrets first, dumps all the helm values files,
/// then helm {operation} all the services.
/// This still might still use helm wait, but it does multiple services at a time.
fn mass_helm(conf: &Config, region: String, umode: UpgradeMode, numjobs: usize) -> Result<()> {
    let mut svcs = vec![];
    for svc in Manifest::available()? {
        debug!("Scanning service {:?}", svc);
        let mf = Manifest::basic(&svc, conf, None)?;
        if !mf.disabled && !mf.external && mf.regions.contains(&region) {
            svcs.push(svc);
        }
    }

    let n_workers = numjobs;
    let n_jobs = svcs.len();
    let pool = ThreadPool::new(n_workers);
    info!("Reconciling {} jobs using {} workers", n_jobs, n_workers);

    let (tx, rx) = channel();
    for svc in svcs {
        // satisfying thread safety
        let mode = umode.clone();
        let reg = region.clone();
        let regdefaults = conf.regions.get(&reg).unwrap().defaults.clone();
        let config = conf.clone();

        let tx = tx.clone(); // tx channel reused in each thread
        pool.execute(move || {
            let v = vault::Vault::default().unwrap(); // TODO:
            let mut mf = Manifest::completed(&reg, &config, &svc, Some(v)).unwrap(); // TODO

            let svc = mf.name.clone();
            // need a tera per service (special folder handling)
            let tera = template::init(&svc).unwrap(); // TODO

            // get version now to limit race condition time
            match helm::infer_version(&svc, &regdefaults) {
                Ok(v) => mf.version = Some(v),
                Err(e) => {
                    return tx.send(Err(e)).expect("channel will be there waiting for the pool");
                }
            }

            let dep = Deployment {
                service: svc.into(),
                region: reg,
                manifest: mf,
                render: Box::new(move |tmpl, context| {
                    template::render(&tera, tmpl, context)
                }),
            };

            // write values to file
            let hfile = format!("{}.helm.gen.yml", dep.service);
            match helm::values(&dep, Some(hfile.clone()), false) {
                Err(e) => {
                    return tx.send(Err(e)).expect("channel will be there waiting for the pool");
                },
                _ => {},
            };
            // upgrade
            let res = helm::upgrade(&dep.manifest, &hfile, mode);
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
