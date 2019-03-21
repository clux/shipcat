use shipcat_definitions::{Config, Region, Team};
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
        svcs.push(shipcat_filebacked::load_manifest(&svc, conf, region)?);
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

    // Reconcile CRDs (definition itself)
    use shipcat_definitions::gen_all_crds;
    for crdef in gen_all_crds() {
        kube::apply_crd(&region.name, crdef.clone(), &region.namespace)?;
    }

    // Make sure config can apply first
    let applycfg = if let Some(ref crs) = &region.customResources {
        // special configtype detected - re-populating config object
        Config::new(crs.shipcatConfig.clone(), &region.name)?.0
    } else {
        config.clone()
    };
    kube::apply_crd(&region.name, applycfg, &region.namespace)?;

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
            let res = crd_reconcile_worker(&svc, &conf, &reg);
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

fn crd_reconcile_worker(svc: &str, conf: &Config, reg: &Region) -> Result<()> {
    let mf = shipcat_filebacked::load_manifest(svc, conf, reg)?;
    kube::apply_crd(svc, mf, &reg.namespace)?;
    Ok(())
}

/// Apply all vault policies in a region
///
/// Generates and writes policies direct to vault using their github team name as auth mappers.
/// Equivalent to:
///
/// ```pseudo
/// for team in shipcat.conf.teams:
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
    let mut svcs = vec![];
    for svc in Manifest::all()? {
        // Can rely on blank manifests in here because metadata is a global property:
        svcs.push(Manifest::blank(&svc)?);
    }
    vault_reconcile(svcs, &conf, reg, n_workers)
}

fn vault_reconcile(svcs: Vec<Manifest>, conf: &Config, region: &Region, n_workers: usize) -> Result<()> {
    use threadpool::ThreadPool;
    use std::sync::mpsc::channel;

    let n_jobs = conf.teams.len();
    let pool = ThreadPool::new(n_workers);
    info!("Starting {} parallel vault jobs using {} workers", n_jobs, n_workers);

    // then parallel apply the remaining ones
    let (tx, rx) = channel();
    for t in conf.teams.clone() {
        let mfs = svcs.clone();
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

fn vault_reconcile_worker(svcs: Vec<Manifest>, team: Team, reg: Region) -> Result<()> {
    use std::path::Path;
    use std::fs::File;
    use std::io::Write;
    //let root = std::env::var("SHIPCAT_MANIFEST_DIR").expect("needs manifest directory set");
    if let Some(admins) = team.clone().vaultAdmins {
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
        debug!("Team '{}' does not have a defined vaultAdmins team in shipcat.conf - ignoring", team.name);
        return Ok(()) // nothing to do
    };
    Ok(())
}
