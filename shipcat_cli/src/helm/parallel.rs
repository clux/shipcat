use threadpool::ThreadPool;
use std::sync::mpsc::channel;
use std::fs;

use super::{Config, Manifest, Region};
use super::{UpgradeMode, UpgradeData};
use super::direct;
use super::helpers;
use super::kube;
use crate::webhooks::{self, UpgradeState};
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
    webhooks::reconcile_event(UpgradeState::Pending, &region);

    // get a list of services for find_redundant_services (done at end)
    let expected : Vec<String> = svcs.iter().map(|mf| mf.name.clone()).collect();

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
    for e in res {
        match e {
            Error(ErrorKind::MissingRollingVersion(svc),_) => {
                // This only happens in rolling envs because version is mandatory in other envs
                warn!("'{}' missing version for {} - please add or install", svc, region.name);
            },
            // remaining cases not ignorable
            _ => {
                webhooks::reconcile_event(UpgradeState::Failed, &region);
                return Err(e)
            },
        }
    }
    webhooks::reconcile_event(UpgradeState::Completed, &region);

    // check for redundant services (informational only for now)
    let _ = helpers::find_redundant_services(&region.namespace, &expected);
    Ok(())
}


/// Parallel reconcile worker that reports information sequentially
///
/// This logs errors and upgrade successes individually.
/// NB: This can reconcile lock-step upgraded services at the moment.
pub fn reconcile_worker(mut mf: Manifest, mode: UpgradeMode, conf: Config, region: Region) -> Result<Option<UpgradeData>> {
    let secret_fail_udata = UpgradeData::pre_install(&mf, &mode);
    mf = match mf.complete(&region) {
        Ok(mfc) => mfc,
        Err(e) => {
            // also fire fail events if secrets fail to resolve
            webhooks::upgrade_event(UpgradeState::Failed, &secret_fail_udata, &region, &conf);
            return Err(e.into());
        },
    };
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
        webhooks::upgrade_event(UpgradeState::Pending, &udata, &region, &conf);

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
                    info!("successfully rolled out {}", &udata.name);
                    // notify about the result directly as they happen
                    webhooks::upgrade_event(UpgradeState::Completed, &udata, &region, &conf);
                } else {
                    error!("Rollout of {} timed out", mf.name);
                    kube::debug(&mf)?;
                    webhooks::upgrade_event(UpgradeState::Failed, &udata, &region, &conf);
                    // need set this as a reconcile level error
                    return Err(ErrorKind::UpgradeTimeout(mf.name.clone(), mf.estimate_wait_time()).into());
                }
            }
        }
    }
    let _ = fs::remove_file(&hfile); // try to remove temporary file
    Ok(upgrade_opt)
}
