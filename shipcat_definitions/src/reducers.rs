use semver::Version;
use std::collections::BTreeMap;
use super::{Result, Config, Manifest};

/// Static reducers over available manifests
impl Manifest {

    /// Find the hardcoded versions of services in a region
    ///
    /// Services without a hardcoded version are not returned.
    pub fn get_versions(conf: &Config, region: &str) -> Result<BTreeMap<String, Version>> {
        use semver::Version;
        let services = Manifest::available()?;
        let mut output = BTreeMap::new();

        for svc in services {
            let mf = Manifest::stubbed(&svc, &conf, &region)?;
            if mf.regions.contains(&region.to_string()) {
                if let Some(v) = mf.version {
                    if let Ok(sv) = Version::parse(&v) {
                        output.insert(svc, sv);
                    }
                }
            }
        }
        Ok(output)
    }

    /// Find the hardcoded images of services in a region
    ///
    /// Services without a hardcoded image will assume the shipcat.conf specific default
    pub fn get_images(conf: &Config, region: &str) -> Result<BTreeMap<String, String>> {
        let services = Manifest::available()?;
        let mut output = BTreeMap::new();

        for svc in services {
            let mf = Manifest::stubbed(&svc, &conf, &region)?;
            if mf.regions.contains(&region.to_string()) {
                if let Some(i) = mf.image {
                    output.insert(svc, i);
                }
            }
        }
        Ok(output)
    }

    /// Generate codeowner strings for each service based based on team owners
    ///
    /// Cross references config.teams with manifest.metadata.team
    /// Each returned string is Github CODEOWNER syntax
    pub fn get_codeowners(conf: &Config, region: &str) -> Result<Vec<String>> {
        let services = Manifest::available()?;
        let mut output = vec![];

        for svc in services {
            let mf = Manifest::stubbed(&svc, &conf, &region)?;
            if let Some(md) = mf.metadata {
                let mut ghids = vec![];
                // unwraps guaranteed by validates on Manifest and Config
                let owners = &conf.teams.iter().find(|t| t.name == md.team).unwrap().owners;
                for o in owners.clone() {
                    ghids.push(format!("@{}", o.github.unwrap()));
                }
                if !owners.is_empty() {
                    output.push(format!("services/{}/* {}", mf.name, ghids.join(" ")));
                }
            }
        }
        Ok(output)
    }

}



#[cfg(test)]
mod tests {
    use tests::setup;
    use super::Config;
    use super::Manifest;
    use semver::Version;

    #[test]
    fn get_versions() {
        setup();
        let conf = Config::read().unwrap();
        let vers = Manifest::get_versions(&conf, "dev-uk").unwrap();

        assert_eq!(vers.len(), 1); // only one of the services has a version
        assert_eq!(vers["fake-ask"], Version::new(1, 6, 0));
    }

    #[test]
    fn get_images() {
        setup();
        let conf = Config::read().unwrap();
        let vers = Manifest::get_images(&conf, "dev-uk").unwrap();

        assert_eq!(vers.len(), 2); // every service gets an image
        assert_eq!(vers["fake-ask"], "quay.io/babylonhealth/fake-ask");
        assert_eq!(vers["fake-storage"], "nginx");
    }

    #[test]
    fn get_codeowners() {
        setup();
        let conf = Config::read().unwrap();
        let cos = Manifest::get_codeowners(&conf, "dev-uk").unwrap();

        assert_eq!(cos.len(), 1); // services without owners get no listing
        assert_eq!(cos[0], "services/fake-ask/* @clux");
    }
}
