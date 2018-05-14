use threadpool::ThreadPool;
use std::sync::mpsc::channel;
use std::fs;

use super::{UpgradeMode, UpgradeData};
use super::direct;
use super::helpers;
use super::{Result, Error, Config, Manifest};


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
    if mf.version.is_none() {
        // get version running now (to limit race condition with deploys)
        let regdefaults = conf.region_defaults(&region)?;
        mf.version = Some(helpers::infer_fallback_version(&svc, &regdefaults)?)
    };

    // Template values file
    let hfile = format!("{}.helm.gen.yml", &svc);
    direct::values(&mf, Some(hfile.clone()))?;

    let upgrade_opt = UpgradeData::new(&mf, &hfile, mode)?;
    if let Some(ref udata) = upgrade_opt {
        // upgrade in given mode, potentially rolling back a failure
        let res = direct::upgrade(&udata).map_err(|e| {
            direct::handle_upgrade_rollbacks(&e, &udata)
        });
        // notify about the result directly as they happen
        direct::handle_upgrade_notifies(res.is_ok(), &udata)?;
    }
    let _ = fs::remove_file(&hfile); // try to remove temporary file
    Ok(upgrade_opt)
}


/// Experimental threaded mass helm operation for components
///
/// Reads secrets first, dumps all the helm values files
/// then helm {operation} all the services.
/// This differs from the above in that it collects the errors at end.
pub fn upgrade_join(svcs: Vec<Manifest>, conf: &Config, region: &str, umode: UpgradeMode, n_workers: usize) -> Result<()> {
    let n_jobs = svcs.len();
    let pool = ThreadPool::new(n_workers);
    info!("Starting {} parallel helm jobs using {} workers", n_jobs, n_workers);

    let udatafallback = if umode == UpgradeMode::UpgradeInstall {
        Some(UpgradeData::from_install(&svcs[0]))
    } else { None };

    let (tx, rx) = channel();
    for mf in svcs {
        // satisfying thread safety
        let mode = umode.clone();
        let reg = region.to_string();
        let config = conf.clone();

        let tx = tx.clone(); // tx channel reused in each thread
        pool.execute(move || {
            info!("Running {} for {}", mode, mf.name);
            let res = upgrade_worker(mf, mode, reg, config);
            tx.send(res).expect("channel will be there waiting for the pool");
        });
    }

    // wait for threads collect errors
    let (errs, upgrades) : (Vec<_>, Vec<_>) = rx.iter().take(n_jobs).partition(Result::is_err);

    if !errs.is_empty() {
        // real errors need to be dealt be propagated here
        // if they occur, then don't try anything fancy - they are not expected
        for eres in &errs {
            if let &Err(ref e) = eres {
                error!("{}", e);
            }
        }
        if let Err(ref e) = errs[0] {
            bail!("Parallel upgade aborted: {} - aborting", e)
        }
    }

    // Handle individual errors
    let mut had_errs = false;
    let mut oks = vec![];
    let mut errs = vec![];
    for r in &upgrades {
        // debug each failed worker - verbose but probably needed
        match r {
            &Ok((None, None)) => debug!("found a blank result - no upgrade needed || diff mode"),
            &Ok((None, Some(ref ud))) => {
                debug!("{} {}", ud.mode, ud.name);
                oks.push(ud.clone());
            },
            &Ok((Some(ref e), Some(ref ud))) => {
                warn!("Failed to {} {}: {}", ud.mode, ud.name, e);
                // TODO: partial rollbacks are pretty dodgy maybe fallback to debug only
                if let Err(e2) = direct::handle_upgrade_rollbacks(e, &ud) {
                    warn!("Failed to handle rollbacks for {}: {}", ud.name, e2);
                }
                errs.push(ud);
                had_errs = true;
            },
            &Ok((Some(ref e), None)) => bail!("Should always have upgrade data: {}", e),
            &Err(ref e) => {
                error!("Failed to {}: {}", umode, e);
                had_errs = true;
            },
        }
    }
    if had_errs {
        // TODO: combine all the errors and notify?
        // currently just notifying about all child services
        for ud in &errs {
            direct::handle_upgrade_notifies(false, ud)?;
        }
        bail!("Failed to parallel upgrade");
    }

    // figure out what happened by looking across all diffs
    let mut svcs = vec![];
    let mut consistent = true;
    let mut prev = None;
    if umode == UpgradeMode::UpgradeInstall {
        // we won't have any diffs here - just make one up
        let ud = udatafallback.unwrap(); // was created in this fn earlier
        direct::handle_upgrade_notifies(true, &ud)?;
        return Ok(())
    }
    for s in &oks {
        svcs.push(s.name.clone());
        if let Some((v1, v2)) = helpers::infer_version_change(&s.diff) {
            if prev == None {
                prev = Some((v1, v2)); // previous is first
            } else if prev != Some((v1, v2)) {
                consistent = false;
            }
        } else if umode != UpgradeMode::UpgradeInstall {
            warn!("Failed to infer version for {}", s.name);
        }
    }
    if consistent && !oks.is_empty() {
        // provide a single slack upgrade notification for consistent components
        let ud = &oks[0];
        direct::handle_upgrade_notifies(true, ud)?;
    } else {
        // notify about all child services as well
        for ud in &oks {
            direct::handle_upgrade_notifies(true, ud)?;
        }
    }
    Ok(())
}

/// A slightly modified direct::upgrade_wrapper
fn upgrade_worker(stubmf: Manifest, mode: UpgradeMode, region: String, conf: Config)
    -> Result<(Option<Error>, Option<UpgradeData>)>
{
    let svc = stubmf.name;
    let mut mf = Manifest::completed(&svc, &conf, &region)?;

    // get version running now (to limit race condition with deploys)
    if stubmf.version.is_some() {
        mf.version = stubmf.version;
    }
    if mf.version.is_none() && mode == UpgradeMode::UpgradeInstall {
        warn!("No version found in either manifest or passed explicitly");
        bail!("helm install needs an explicit version")
    }
    // Other modes can infer in a pinch
    if mf.version.is_none() {
        let regdefaults = conf.region_defaults(&region)?;
        mf.version = Some(helpers::infer_fallback_version(&svc, &regdefaults)?);
    };

    // Template values file
    let hfile = format!("{}.helm.gen.yml", &svc);
    direct::values(&mf, Some(hfile.clone()))?;

    let upgrade_opt = UpgradeData::new(&mf, &hfile, mode)?;
    // Upgrades after this point can happen if services are failing
    // deal with these expected errors seperately:
    match upgrade_opt {
        Some(udata) => {
            match direct::upgrade(&udata) {
                Err(e) => return Ok((Some(e), Some(udata))), // Ok, upgrade did fail
                Ok(_) => return Ok((None, Some(udata))), // Ok, upgrade succeeded
            }
        },
        None => return Ok((None, None)),  // Ok, no upgraded was performed
    }
}
