use super::{Result, ResultExt, ErrorKind, Config, Manifest};

/// Validate the manifest of a service in the services directory
///
/// This will populate the manifest for all supported environments,
/// and `verify` their parameters.
/// Optionally, it will also verify that all secrets are found in the corresponding
/// vault locations serverside (which require vault credentials).
pub fn manifest(services: Vec<String>, conf: &Config, region: String, secrets: bool) -> Result<()> {
    for svc in services {
        let mut tmpmf = Manifest::basic(&svc, conf, Some(region.clone()))?;
        if tmpmf.regions.contains(&region) {
            info!("validating {} for {}", svc, region);
            let mf = if secrets {
                Manifest::completed(&svc, conf, &region)?
            } else {
                // ensure we also verify template against mocked secrets
                Manifest::mocked(&svc, conf, &region)?
            };
            mf.verify(conf).chain_err(|| ErrorKind::ManifestVerifyFailure(svc.clone()))?;
            info!("validated {} for {}", svc, region);
        } else if tmpmf.external {
            tmpmf.verify(&conf)?; // exits early - but will verify some stuff
        } else {
            bail!("{} is not configured to be deployed in {}", svc, region)
        }
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
        reg.verify_secrets_exist(&r)?; // verify secrets for the region
        let services = Manifest::available()?;
        for svc in services {
            let mut mf = Manifest::basic(&svc, conf, Some(r.clone()))?;
            if mf.regions.contains(&r) && !mf.external && !mf.disabled {
                info!("validating secrets for {} in {}", svc, r);
                mf.fill(&conf, &r)?;
                mf.verify_secrets_exist(&reg.vault)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tests::setup;
    use super::manifest as validate;
    use super::Config;

    #[test]
    fn validate_test() {
        setup();
        let conf = Config::read().unwrap();
        let res = validate(vec!["fake-ask".into()], &conf, "dev-uk".into(), true);
        assert!(res.is_ok());
        let res2 = validate(vec!["fake-storage".into(), "fake-ask".into()], &conf, "dev-uk".into(), false);
        assert!(res2.is_ok())
    }
}
