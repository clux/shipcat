use threadpool::ThreadPool;
use std::sync::mpsc::channel;
use std::fs;

use super::{UpgradeMode, UpgradeData};
use super::direct;
use super::helpers;
use super::{Result, Config, Manifest};


/// Stable threaded mass helm operation
///
/// Reads secrets first, dumps all the helm values files
/// then helm {operation} all the services.
/// The helm operations does --wait for upgrades, but this parallelises the wait
/// and catches any errors.
/// All operations run to completion and the first error is returned at end if any.
pub fn reconcile(svcs: Vec<Manifest>, conf: &Config, region: &str, umode: UpgradeMode, n_workers: usize) -> Result<()> {
    let n_jobs = svcs.len();
    let pool = ThreadPool::new(n_workers);
    info!("Starting {} parallel helm jobs using {} workers", n_jobs, n_workers);

    let (tx, rx) = channel();
    for mf in svcs {
        // satisfying thread safety
        let mode = umode.clone();
        let reg = region.into();
        let config = conf.clone();

        let tx = tx.clone(); // tx channel reused in each thread
        pool.execute(move || {
            info!("Running {} for {}", mode, mf.name);
            let res = reconcile_worker(mf, mode, reg, config);
            tx.send(res).expect("channel will be there waiting for the pool");
        });
    }

    // wait for threads collect errors
    let res = rx.iter().take(n_jobs).map(|r| {
        match &r {
            &Ok(Some(ref ud)) => debug!("{} {}", ud.mode, ud.name),
            &Ok(None) => {},
            &Err(ref e) => error!("Failed to {}: {}", umode, e),
        }
        r
    }).filter_map(Result::err).collect::<Vec<_>>();

    // propagate first error if exists
    if !res.is_empty() {
        bail!("{}", res[0]);
    }
    Ok(())
}


/// Parallel reconcile worker that reports information sequentially
///
/// This logs errors and upgrade successes individually.
/// NB: This can reconcile lock-step upgraded services at the moment.
fn reconcile_worker(tmpmf: Manifest, mode: UpgradeMode, region: String, conf: Config) -> Result<Option<UpgradeData>> {
    let svc = tmpmf.name;

    let mut mf = Manifest::completed(&svc, &conf, &region)?;

    let regdata = &conf.regions[&region];
    // get version running now (to limit race condition with deploys)
    // this query also lets us detect if we have to install or simply upgrade
    let (exists, fallback) = match helpers::infer_fallback_version(&svc, &regdata.namespace) {
        Ok(running_ver) => (true, running_ver),
        Err(e) => {
            if let Some(v) = mf.version.clone() {
                warn!("Service {} will be installed by reconcile", mf.name);
                if mode == UpgradeMode::DiffOnly {
                    return Ok(None);
                }
                (false, v)
            } else {
                warn!("Service {} has no version specified in manifest and is not installed", mf.name);
                error!("Service {} must be installed manually in rolling environment", mf.name);
                return Err(e);
            }
        }
    };
    debug!("reconcile worker {} - exists?{} fallback:{}", mf.name, exists, fallback);

    // only override version if not in manifests
    if mf.version.is_none() {
         mf.version = Some(fallback)
    };

    // Template values file
    let hfile = format!("{}.helm.gen.yml", &svc);
    direct::values(&mf, Some(hfile.clone()))?;

    let upgrade_opt = UpgradeData::new(&mf, &hfile, mode, exists)?;
    if let Some(ref udata) = upgrade_opt {
        // upgrade in given mode, potentially rolling back a failure
        let res = direct::upgrade(&udata);
        // notify about the result directly as they happen
        let _ = direct::handle_upgrade_notifies(res.is_ok(), &udata).map_err(|e| {
            warn!("Failed to slack notify about upgrade: {}", e);
            e
        });
        if let Err(e) = res {
            direct::handle_upgrade_rollbacks(&e, &udata, &mf)?;
            return Err(e);
        }
    }
    let _ = fs::remove_file(&hfile); // try to remove temporary file
    Ok(upgrade_opt)
}
