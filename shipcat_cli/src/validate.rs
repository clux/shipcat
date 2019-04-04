use super::{Config, Region};
use super::Result;

/// Validate the manifest of a service in the services directory
///
/// This will populate the manifest for all supported environments,
/// and `verify` their parameters.
/// Optionally, it will also verify that all secrets are found in the corresponding
/// vault locations serverside (which require vault credentials).
pub fn manifest(services: Vec<String>, conf: &Config, reg: &Region, secrets: bool) -> Result<()> {
    conf.verify()?; // this should work even with a limited config!
    conf.verify_version_pin(&reg.environment)?;
    for svc in services {
        info!("validating {} for {}", svc, reg.name);
        let mf = if secrets {
            shipcat_filebacked::load_manifest(&svc, conf, reg)?.complete(reg)?
        } else {
            shipcat_filebacked::load_manifest(&svc, conf, reg)?.stub(reg)?
        };
        mf.verify(conf, reg)?;
        info!("validated {} for {}", svc, reg.name);
    }
    Ok(())
}

/// Validate the secrets exists in all regions
///
/// This is one of very few functions not validating a single kube context,
/// so it does special validation of all the regions.
pub fn secret_presence(conf: &Config, regions: Vec<String>) -> Result<()> {
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

/// A config verifier
///
/// This works with Base configs and File configs
/// Manifest repositories should verify with the full file configs for all the sanity.
pub fn config(conf: Config) -> Result<()> {
    conf.verify()?;
    Ok(())
}
