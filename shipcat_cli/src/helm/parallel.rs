use threadpool::ThreadPool;
use std::sync::mpsc::channel;
use std::fs;

use super::{Config, Manifest, Region};
use super::{UpgradeMode, UpgradeState, UpgradeData};
use super::direct;
use super::audit::audit_reconciliation;
use super::helpers;
use super::kube;
use super::{Result, Error, ErrorKind};


/// Stable threaded mass helm operation
///
/// Reads secrets first, dumps all the helm values files
/// then helm {operation} all the services.
/// The helm operations does --wait for upgrades, but this parallelises the wait
/// and catches any errors.
/// All operations run to completion and the first error is returned at end if any.
pub fn reconcile(svcs: Vec<Manifest>, conf: &Config, region: &Region, umode: UpgradeMode, n_workers: usize) -> Result<()> {
    let n_jobs = svcs.len();
    let pool = ThreadPool::new(n_workers);
    info!("Starting {} parallel helm jobs using {} workers", n_jobs, n_workers);

    if let Some(ref webhooks) = &region.webhooks {
        if let Err(e) = audit_reconciliation(&UpgradeState::Pending, &region.name, &webhooks.audit) {
            warn!("Failed to notify about reconcile: {}", e);
        }
    }

    let (tx, rx) = channel();
    for mf in svcs {
        // satisfying thread safety
        let mode = umode.clone();
        let reg = region.clone();
        let config = conf.clone();


        let tx = tx.clone(); // tx channel reused in each thread
        pool.execute(move || {
            info!("Running {} for {}", mode, mf.name);
            let res = reconcile_worker(mf, mode, config, reg);
            tx.send(res).expect("channel will be there waiting for the pool");
        });
    }

    // wait for threads collect errors
    let res = rx.iter().take(n_jobs).map(|r| {
        match &r {
            &Ok(Some(ref ud)) => debug!("{} {}", ud.mode, ud.name),
            &Ok(None) => {},
            &Err(ref e) => warn!("{} error: {}", umode, e),
        }
        r
    }).filter_map(Result::err).collect::<Vec<_>>();

    // propagate first non-ignorable error if exists
    let mut anyNonIgnorableError = false;
    for e in res {
        match e {
            Error(ErrorKind::MissingRollingVersion(svc),_) => {
                // This only happens in rolling envs because version is mandatory in other envs
                warn!("'{}' missing version for {} - please add or install", svc, region.name);
            },
            // remaining cases not ignorable
            e => {
                anyNonIgnorableError = true;
                return Err(e)
            },
        }
    }

    if let Some(ref webhooks) = &region.webhooks {
        let us = if anyNonIgnorableError { UpgradeState::Failed }
                                    else { UpgradeState::Completed };
        if let Err(e) = audit_reconciliation(&us, &region.name, &webhooks.audit) {
            warn!("Failed to notify about reconcile: {}", e);
        }
    }

    Ok(())
}


/// Parallel reconcile worker that reports information sequentially
///
/// This logs errors and upgrade successes individually.
/// NB: This can reconcile lock-step upgraded services at the moment.
fn reconcile_worker(mut mf: Manifest, mode: UpgradeMode, _conf: Config, region: Region) -> Result<Option<UpgradeData>> {
    mf = mf.complete(&region)?;
    let svc = mf.name.clone();

    // get version running now (to limit race condition with deploys)
    // this query also lets us detect if we have to install or simply upgrade
    let (exists, fallback) = match helpers::infer_fallback_version(&svc, &region.namespace) {
        Ok(running_ver) => (true, running_ver),
        Err(e) => {
            if let Some(v) = mf.version.clone() {
                warn!("Service {} will be installed by reconcile", mf.name);
                if mode == UpgradeMode::DiffOnly {
                    return Ok(None);
                }
                (false, v)
            } else {
                warn!("ignoring service {} without version as it is not installed in rolling environment", mf.name);
                return Err(e.chain_err(|| ErrorKind::MissingRollingVersion(mf.name.clone())));
            }
        }
    };
    debug!("reconcile worker {} - exists?{} fallback:{}", mf.name, exists, fallback);

    // only override version if not in manifests
    if mf.version.is_none() {
         mf.version = Some(fallback)
    };
    // sanity verify what we changed (no-shoehorning in illegal versions in rolling envs)
    region.versioningScheme.verify(&mf.version.clone().unwrap())?;


    // Template values file
    let hfile = format!("{}.helm.gen.yml", &svc);
    direct::values(&mf, Some(hfile.clone()))?;

    let upgrade_opt = UpgradeData::new(&mf, &hfile, mode, exists)?;
    if let Some(ref udata) = upgrade_opt {
        let _ = direct::handle_upgrade_notifies(&UpgradeState::Pending, &udata, &region).map_err(|e| {
            warn!("Failed to notify about upgrade: {}", e);
            e
        });

        // upgrade in given mode, potentially rolling back a failure
        match direct::upgrade(&udata) {
            Err(e) => {
                // upgrade failed immediately - couldn't create resources
                kube::debug(&mf)?;
                error!("{} from {}", e, udata.name);
                return Err(e);
            }
            Ok(_)  => {
                // after helm upgrade / kubectl apply, check rollout status in a loop:
                if kube::await_rollout_status(&mf)? {
                    // notify about the result directly as they happen
                    let _ = direct::handle_upgrade_notifies(&UpgradeState::Completed, &udata, &region).map_err(|e| {
                        warn!("Failed to notify about upgrade: {}", e);
                        e
                    });
                } else {
                    error!("Rollout of {} timed out", mf.name);
                    kube::debug(&mf)?;
                    let _ = direct::handle_upgrade_notifies(&UpgradeState::Failed, &udata, &region).map_err(|e| {
                        warn!("Failed to notify about upgrade: {}", e);
                        e
                    });
                    // need set this as a reconcile level error
                    return Err(ErrorKind::UpgradeTimeout(mf.name.clone(), mf.estimate_wait_time()).into());
                }
            }
        }
    }
    let _ = fs::remove_file(&hfile); // try to remove temporary file
    Ok(upgrade_opt)
}
