use super::{Config, Region};
use super::{Result, Error};
use rayon::{iter::Either, prelude::*};
use crate::git;


/// Validate all manifests in a service directory for a region
///
/// This is meant to replace `shipcat validate ..all_services`
/// This does not check secrets.
pub fn regional_manifests(conf: &Config, reg: &Region) -> Result<()> {
    let available = shipcat_filebacked::available(conf, &reg)?;

    let (errs, _mfs) : (Vec<Error>, Vec<_>) = available.par_iter()
        .map(|mf| {
            let mf = shipcat_filebacked::load_manifest(&mf.base.name, &conf, &reg)?
                .stub(&reg)?;
            mf.verify(&conf, &reg)?;
            Ok(mf)
        })
        .partition_map(Either::from);
    if !errs.is_empty() {
        for e in &errs {
            error!("{}", e);
            debug!("{:?}", e);
        }
        bail!("Invalid shipcat data in {} files", errs.len());
    }
    // TODO: cross reference uniqueness values here
    Ok(())
}

/// Validate all manifests in a service directory for ALL regions
///
/// This is meant to replace a for loop over shipcat list-regions
/// This does not check secrets
pub fn all_manifests() -> Result<()> {
    use crate::ConfigType;
    let regions = Config::read()?.list_regions();

    let errs = regions.par_iter()
        .map(|r| {
            let (conf, region) = Config::new(ConfigType::Base, &r)?;
            regional_manifests(&conf, &region)?;
            Ok(())
        })
        .filter_map(Result::err)
        .collect::<Vec<_>>();

    if !errs.is_empty() {
        for e in &errs {
            error!("{}", e);
            debug!("{:?}", e);
        }
        bail!("Invalid shipcat data in {} files", errs.len());
    }
    Ok(())
}


/// Validate the manifest of a service in the services directory
///
/// This will populate the manifest for all supported environments,
/// and `verify` their parameters.
/// Optionally, it will also verify that all secrets are found in the corresponding
/// vault locations serverside (which require vault credentials).
pub fn manifest(services: Vec<String>, conf: &Config, reg: &Region, secrets: bool) -> Result<()> {
    conf.verify()?; // this should work even with a limited config!
    for svc in services {
        debug!("validating {} for {}", svc, reg.name);
        let mf = if secrets {
            shipcat_filebacked::load_manifest(&svc, conf, reg)?.complete(reg)?
        } else {
            shipcat_filebacked::load_manifest(&svc, conf, reg)?.stub(reg)?
        };
        mf.verify(conf, reg)?;
        debug!("validated {} for {}", svc, reg.name);
    }
    Ok(())
}

/// Validate the secrets exists in all regions
///
/// This is one of very few functions not validating a single kube context,
/// so it does special validation of all the regions.
pub fn secret_presence_full(conf: &Config, regions: Vec<String>) -> Result<()> {
    for r in regions {
        info!("validating secrets in {}", r);
        let reg = conf.get_region(&r)?; // verifies region or region alias exists
        reg.verify_secrets_exist()?; // verify secrets for the region
        for svc in shipcat_filebacked::available(conf, &reg)? {
            let mf = shipcat_filebacked::load_manifest(&svc.base.name, conf, &reg)?;
            debug!("validating secrets for {} in {}", &svc.base.name, r);
            mf.verify_secrets_exist(&reg.vault)?;
        }
    }
    Ok(())
}

/// Validate the secrets exists in all regions for a subset of services
///
/// This is an optimization of secret_presence_git
pub fn secret_presence_explicit(svcs: Vec<String>, conf: &Config, regions: Vec<String>) -> Result<()> {
    for r in regions {
        info!("validating secrets in {}", r);
        let reg = conf.get_region(&r)?; // verifies region or region alias exists
        reg.verify_secrets_exist()?; // verify secrets for the region
        debug!("Validating {:?}", svcs);
        for svc in &svcs {
            debug!("Validating {}", svc);
            if let Ok(mf) = shipcat_filebacked::load_manifest(&svc, conf, &reg) {
                if !mf.regions.contains(&r) {
                    debug!("ignoring {} for {} (not deployed there)", svc, r);
                    continue;
                }
                debug!("validating secrets for {} in {}", &svc, r);
                mf.verify_secrets_exist(&reg.vault)?;
            }
        }
    }
    Ok(())
}

/// Validate secrets exists in all regions, but only for services touched in git
pub fn secret_presence_git(conf: &Config, regions: Vec<String>) -> Result<()> {
    for r in regions {
        info!("validating secrets in {}", r);
        let reg = conf.get_region(&r)?; // verifies region or region alias exists
        reg.verify_secrets_exist()?; // verify secrets for the region

        // Try to find services changed by git:
        let svcs = match git_diff_changes() {
            Ok(svcs) => svcs,
            // if that for some reason fails, then do all services for that region
            Err(e) => {
                warn!("Error from git: {}", e);
                warn!("Falling back to a full validate");
                shipcat_filebacked::available(conf, &reg)?.into_iter().map(|s| s.base.name).collect()
            },
        };
        for svc in svcs {
            if let Ok(mf) = shipcat_filebacked::load_manifest(&svc, conf, &reg) {
                if !mf.regions.contains(&r) {
                    debug!("ignoring {} for {} (not deployed there)", svc, r);
                    continue;
                }
                debug!("validating secrets for {} in {}", &svc, r);
                mf.verify_secrets_exist(&reg.vault)?;
            }
        }

    }
    Ok(())
}

/// A config verifier
///
/// This works with Base configs and File configs
/// Manifest repositories should verify with the full file configs for all the sanity.
pub fn config(conf: Config) -> Result<()> {
    conf.verify()?;
    Ok(())
}


// Dumb git diff helper that matches normal service files:
//
// Effectively checks:
// git diff --name-only $(git merge-base origin/master HEAD) | grep ./services/{svc}/*
fn git_diff_changes() -> Result<Vec<String>> {
    use regex::Regex;
    let merge_base = git::merge_base()?;
    let diff_output = git::diff_filenames(&merge_base)?;
    let svc_re = Regex::new(r"^services/(?P<svc>[0-9a-z\-]{1,50})/").unwrap();
    let mut res = vec![];
    for l in diff_output.lines() {
        if let Some(caps) = svc_re.captures(l) {
            if let Some(svc) = caps.name("svc") {
                res.push(svc.as_str().to_string());
            }
        }
    }
    Ok(res)
}
