use threadpool::ThreadPool;
use std::sync::mpsc::channel;

use super::vault;
use super::generate::Deployment;
use super::template;

use super::UpgradeMode;
use super::direct;
use super::helpers;
use super::{Result, Config, Manifest};


pub fn upgrade(svcs: Vec<String>, conf: &Config, region: String, umode: UpgradeMode, n_workers: usize) -> Result<()> {
    let n_jobs = svcs.len();
    let pool = ThreadPool::new(n_workers);
    info!("Starting {} parallel helm jobs using {} workers", n_jobs, n_workers);

    let (tx, rx) = channel();
    for svc in svcs {
        // satisfying thread safety
        let mode = umode.clone();
        let reg = region.clone();
        let config = conf.clone();

        let tx = tx.clone(); // tx channel reused in each thread
        pool.execute(move || {
            info!("Running {} for {}", mode, svc);
            let res = upgrade_worker(svc, mode, reg, config);
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


// The work that will be done in parallel
fn upgrade_worker(svc: String, mode: UpgradeMode, region: String, conf: Config) -> Result<(Manifest, String)> {
    // Create a vault client and fetch all secrets
    let v = vault::Vault::default()?;
    let mut mf = Manifest::completed(&region, &conf, &svc, Some(v))?;

    // instantiate a tera templating service (special folder handling per svc)
    let tera = template::init(&svc)?;

    // get version running now (to limit race condition with deploys)
    let regdefaults = conf.regions.get(&region).unwrap().defaults.clone();
    mf.version = if let Some(v) = mf.version {
        // If pinned in manifests, use that version
        Some(v)
    } else {
        Some(helpers::infer_fallback_version(&svc, &regdefaults)?)
    };

    // Template values file
    let hfile = format!("{}.helm.gen.yml", &svc);
    let dep = Deployment {
        service: svc.into(),
        region: region,
        manifest: mf,
        render: Box::new(move |tmpl, context| {
            template::render(&tera, tmpl, context)
        }),
    };
    direct::values(&dep, Some(hfile.clone()), false)?;
    direct::upgrade(&dep.manifest, &hfile, mode)
}
