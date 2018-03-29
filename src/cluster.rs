use super::generate;
use super::helm;
use super::helm::{UpgradeMode};
use super::{Result, Config, Manifest};

/// Experimental reconcile that is parallelised
///
/// This still uses helm wait, but it does multiple services at a time.
pub fn helm_diff(conf: &Config, region: String) -> Result<()> {
    use super::vault;
    use super::template;
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
            let dep = generate::Deployment {
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
    use threadpool::ThreadPool;
    use std::sync::mpsc::channel;

    let n_workers = 8;
    let n_jobs = manifests.len();
    let pool = ThreadPool::new(n_workers);
    info!("Reconciling {} jobs using {} workers", n_jobs, n_workers);

    let (tx, rx) = channel();
    for mf in manifests {
        let tx = tx.clone();
        pool.execute(move|| {
            let mode = UpgradeMode::DiffOnly;
            let hfile = format!("{}.helm.gen.yml", mf.name); // as above
            let res = helm::upgrade(&mf, &hfile, mode);
            tx.send(res).expect("channel will be there waiting for the pool");
        });
    }
    let _ = rx.iter().take(n_jobs).map(|r| {
        match &r {
            &Ok((ref mf, _)) => debug!("diffed {}", mf.name), // TODO: s/Diffed/Reconciled once !dryrun
            &Err(ref e) => error!("Failed to reconcile {}", e)
        }
        r
    }).collect::<Vec<_>>();

    Ok(())
}
