use super::{Backend, Config, Manifest, Region};
use super::Result;

/// Validate the manifest of a service in the services directory
///
/// This will populate the manifest for all supported environments,
/// and `verify` their parameters.
/// Optionally, it will also verify that all secrets are found in the corresponding
/// vault locations serverside (which require vault credentials).
pub fn manifest(services: Vec<String>, conf: &Config, reg: &Region, secrets: bool) -> Result<()> {
    for svc in services {
        info!("validating {} for {}", svc, reg.name);
        let mf = if secrets {
            Manifest::completed(&svc, conf, reg)?
        } else {
            Manifest::stubbed(&svc, conf, reg)?
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
        let (_, mut reg) = conf.get_region(&r)?; // verifies region or region alias exists
        reg.verify_secrets_exist()?; // verify secrets for the region
        for svc in Manifest::available(&reg.name)? {
            let mut mf = Manifest::base(&svc, conf, &reg)?;
            debug!("validating secrets for {} in {}", svc, r);
            mf.verify_secrets_exist(&reg.vault)?;
        }
    }
    Ok(())
}
