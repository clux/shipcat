use super::{Config, Manifest, Region};
use super::Result;

/// Validate the manifest of a service in the services directory
///
/// This will populate the manifest for all supported environments,
/// and `verify` their parameters.
/// Optionally, it will also verify that all secrets are found in the corresponding
/// vault locations serverside (which require vault credentials).
pub fn manifest(services: Vec<String>, conf: &Config, reg: &Region, secrets: bool) -> Result<()> {
    conf.verify()?; // this should work even with a limited config!
    for svc in services {
        info!("validating {} for {}", svc, reg.name);
        let mf = if secrets {
            Manifest::base(&svc, conf, reg)?.complete(reg)?
        } else {
            Manifest::base(&svc, conf, reg)?.stub(reg)?
        };
        mf.verify(conf, reg)?;
        info!("validated {} for {}", svc, reg.name);
    }
    Ok(())
}

/// A config verifier
///
/// This works with Base configs and File configs
/// Manifest repositories should verify with the full file configs for all the sanity.
pub fn config(conf: &Config) -> Result<()> {
    conf.verify()?;
    Ok(())
}

/// Print the config
///
/// This allows debugging the config type after filtering/completing
pub fn show_config(conf: &Config) -> Result<()> {
    conf.print()?;
    Ok(())
}

/// Validate the secrets exists in all regions
///
/// This is one of very few functions not validating a single kube context,
/// so it does special validation of all the regions.
pub fn secret_presence(conf: &Config, regions: Vec<String>) -> Result<()> {
    for r in regions {
        info!("validating secrets in {}", r);
        let mut reg = conf.get_region(&r)?; // verifies region or region alias exists
        reg.verify_secrets_exist()?; // verify secrets for the region
        for svc in Manifest::available(&reg.name)? {
            let mut mf = Manifest::base(&svc, conf, &reg)?;
            debug!("validating secrets for {} in {}", svc, r);
            mf.verify_secrets_exist(&reg.vault)?;
        }
    }
    Ok(())
}
