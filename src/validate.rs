use super::vault::Vault;
use super::{Result, Config, Manifest};

/// Validate the manifest of a service in the services directory
///
/// This will populate the manifest for all supported environments,
/// and `verify` their parameters.
/// Optionally, it will also verify that all secrets are found in the corresponding
/// vault locations serverside (which require vault credentials).
pub fn manifest(services: Vec<String>, conf: &Config, region: String, vault: Option<Vault>) -> Result<()> {
    for svc in services {
        let mut mf = Manifest::basic(&svc, conf, Some(region.clone()))?;
        if mf.regions.contains(&region) {
            info!("validating {} for {}", svc, region);
            mf.fill(&conf, &region, &vault)?;
            mf.verify(&conf)?;
            info!("validated {} for {}", svc, region);
            mf.print()?; // print it if sufficient verbosity
        } else if mf.external {
            mf.verify(&conf)?; // exits early - but will verify some stuff
        } else {
            bail!("{} is not configured to be deployed in {}", svc, region)
        }
    }
    Ok(())
}

// Validate the secrets exists in all regions
pub fn secret_presence(conf: &Config, regions: Vec<String>) -> Result<()> {
    for r in regions {
        info!("validating secrets in {}", r);
        let services = Manifest::available()?;
        for svc in services {
            let mut mf = Manifest::basic(&svc, conf, Some(r.clone()))?;
            if mf.regions.contains(&r) && !mf.external && !mf.disabled {
                info!("validating secrets for {} in {}", svc, r);
                mf.fill(&conf, &r, &None)?;
                mf.verify_secrets_exist(&r)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tests::setup;
    use super::manifest as validate;
    use super::Vault;
    use super::Config;

    #[test]
    fn validate_test() {
        setup();
        let client = Vault::default().unwrap();
        let conf = Config::read().unwrap();
        let res = validate(vec!["fake-ask".into()], &conf, "dev-uk".into(), Some(client));
        assert!(res.is_ok());
        let res2 = validate(vec!["fake-storage".into(), "fake-ask".into()], &conf, "dev-uk".into(), None);
        assert!(res2.is_ok())
    }
}
