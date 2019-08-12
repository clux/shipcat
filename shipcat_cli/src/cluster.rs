use threadpool::ThreadPool;
use std::sync::mpsc::channel;

use shipcat_definitions::{Config, Region, Team, BaseManifest};
use shipcat_filebacked::{SimpleManifest};

use crate::apply;
use crate::helm; // temporary
use super::kubectl;
use crate::webhooks::{self, UpgradeState};
use super::{Result, Error, ErrorKind};

/// Apply all crds in a region
///
/// Temporary helper that shells out to kubectl apply in parallel.
/// This will go away with catapult.
pub fn mass_crd(conf_sec: &Config, conf_base: &Config, reg: &Region, n_workers: usize) -> Result<()> {
    let svcs = shipcat_filebacked::available(conf_base, reg)?;
    crd_reconcile(svcs, conf_sec, conf_base, &reg.name, n_workers)
}

fn crd_reconcile(svcs: Vec<SimpleManifest>,
    config_sec: &Config, config_base: &Config,
    region: &str,
    n_workers: usize) -> Result<()>
{
    // NB: This needs config_base for base crd application
    // shipcatconfig crd should not have secrets when applied
    // shipcatmanifest_crd should not have secrets when applied (fine as long as manifest is not complete())
    // but when doing the actual upgrade we need a config + region with secrets.
    assert!(config_sec.has_secrets());
    assert!(!config_base.has_secrets());
    let region_sec = config_sec.get_regions().iter().find(|r| r.name == region).unwrap().clone();
    let region_base = config_base.get_regions().iter().find(|r| r.name == region).unwrap().clone();

    webhooks::reconcile_event(UpgradeState::Pending, &region_sec);
    // Reconcile CRDs (definition itself)
    use shipcat_definitions::gen_all_crds;
    for crdef in gen_all_crds() {
        kubectl::apply_crd(&region_base.name, crdef.clone(), &region_base.namespace)?;
    }

    // Make sure config can apply first
    let applycfg = if let Some(ref crs) = &region_base.customResources {
        // special configtype detected - re-populating config object
        Config::new(crs.shipcatConfig.clone(), &region_base.name)?.0
    } else {
        config_base.clone()
    };
    kubectl::apply_crd(&region_base.name, applycfg, &region_base.namespace)?;

    // Single instruction kubectl delete shipcat manifests .... of excess ones
    let svc_names = svcs.iter().map(|x| x.base.name.to_string()).collect::<Vec<_>>();
    kubectl::remove_redundant_manifests(&region_sec.namespace, &svc_names)?;

    let n_jobs = svcs.len();
    let pool = ThreadPool::new(n_workers);
    info!("Starting {} parallel kube jobs using {} workers", n_jobs, n_workers);

    webhooks::reconcile_event(UpgradeState::Started, &region_sec);
    // then parallel apply the remaining ones
    let force = std::env::var("SHIPCAT_MASS_RECONCILE").unwrap_or("0".into()) == "1";
    let wait_for_rollout = true;
    let (tx, rx) = channel();
    for svc in svc_names.clone() {
        let reg = region_sec.clone();
        let conf = config_sec.clone();
        assert!(conf.has_secrets());

        let tx = tx.clone(); // tx channel reused in each thread
        pool.execute(move || {
            debug!("Running CRD reconcile for {:?}", svc);
            let res = apply::apply(&svc, force, &reg, &conf, wait_for_rollout, None);
            tx.send(res).expect("channel will be there waiting for the pool");
        });
    }
    // wait for threads collect errors
    let res = rx.iter().take(n_jobs).map(|r| {
        match r {
            Ok(_) => {},
            Err(ref e) => warn!("{}", e),
        }
        r
    }).filter_map(Result::err).collect::<Vec<_>>();

    // propagate first non-ignorable error if exists
    for e in res {
        match e {
            Error(ErrorKind::MissingRollingVersion(svc),_) => {
                // This only happens in rolling envs because version is mandatory in other envs
                warn!("'{}' missing version for {} - please add or install", svc, region_sec.name);
            },
            // remaining cases not ignorable
            _ => {
                webhooks::reconcile_event(UpgradeState::Failed, &region_sec);
                return Err(e)
            },
        }
    }

    // Otherwise we're good
    webhooks::reconcile_event(UpgradeState::Completed, &region_sec);

    // temporary sanity help - to clean out stragglers
    let _ = helm::find_redundant_services(&region_sec.namespace, &svc_names);
    Ok(())
}

/// Apply all vault policies in a region
///
/// Generates and writes policies direct to vault using their github team name as auth mappers.
/// Equivalent to:
///
/// ```pseudo
/// for team in config.teams:
///   shipcat get vaultpolicy {team.name} | vault policy write {team.admins} -
///   vault write auth/github/map/teams/{team.admins} value={team.admins}
/// ```
///
/// using vault setup for the vault specified in the `Region`.
/// If one vault is reused for all regions, this can be done once.
///
/// Requires a `vault login` outside of this command as a user who
/// is sufficiently elevated to write general policies.
pub fn mass_vault(conf: &Config, reg: &Region, n_workers: usize) -> Result<()> {
    let svcs = shipcat_filebacked::all(conf)?;
    vault_reconcile(svcs, &conf, reg, n_workers)
}

fn vault_reconcile(mfs: Vec<BaseManifest>, conf: &Config, region: &Region, n_workers: usize) -> Result<()> {
    let n_jobs = conf.teams.len();
    let pool = ThreadPool::new(n_workers);
    info!("Starting {} parallel vault jobs using {} workers", n_jobs, n_workers);

    // then parallel apply the remaining ones
    let (tx, rx) = channel();
    for t in conf.teams.clone() {
        let mfs = mfs.clone();
        let reg = region.clone();
        let tx = tx.clone(); // tx channel reused in each thread
        pool.execute(move || {
            debug!("Running vault reconcile for {}", t.name);
            let res = vault_reconcile_worker(mfs, t, reg);
            tx.send(res).expect("channel will be there waiting for the pool");
        });
    }
    // wait for threads collect errors
    let res = rx.iter().take(n_jobs).map(|r| {
        match r {
            Ok(_) => {},
            Err(ref e) => warn!("error: {}", e),
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

fn vault_reconcile_worker(svcs: Vec<BaseManifest>, team: Team, reg: Region) -> Result<()> {
    use std::path::Path;
    use std::fs::File;
    use std::io::Write;
    //let root = std::env::var("SHIPCAT_MANIFEST_DIR").expect("needs manifest directory set");
    if let Some(admins) = team.clone().githubAdmins {
        // TODO: validate that the github team exists?
        let policy = reg.vault.make_policy(svcs, team.clone(), reg.environment.clone())?;
        debug!("Vault policy: {}", policy);
        // Write policy to a file named "{admins}-policy.hcl"
        let pth = Path::new(".").join(format!("{}-policy.hcl", admins));
        info!("Writing vault policy for {} to {}", admins, pth.display());
        let mut f = File::create(&pth)?;
        writeln!(f, "{}", policy)?;
        // Write a vault policy with the name equal to the admin team:
        use std::process::Command;
        // vault write policy < file
        {
            info!("Applying vault policy for {} in {}", admins, reg.name);
            let write_args = vec![
                "policy".into(),
                "write".into(),
                admins.clone(),
                format!("{}-policy.hcl", admins),
            ];
            debug!("vault {}", write_args.join(" "));
            let s = Command::new("vault").args(&write_args).status()?;
            if !s.success() {
                bail!("Subprocess failure from vault: {}", s.code().unwrap_or(1001))
            }
        }
        // vault write auth -> team
        {
            info!("Associating vault policy for {} with github team {} in {}", team.name, admins, reg.name);
            let assoc_args = vec![
                "write".into(),
                format!("auth/github/map/teams/{}", admins),
                format!("value={}", admins),
            ];
            debug!("vault {}", assoc_args.join(" "));
            let s = Command::new("vault").args(&assoc_args).status()?;
            if !s.success() {
                bail!("Subprocess failure from vault: {}", s.code().unwrap_or(1001))
            }
        }
    } else {
        debug!("Team '{}' does not have a defined githubAdmins team in shipcat.conf - ignoring", team.name);
        return Ok(()) // nothing to do
    };
    Ok(())
}
