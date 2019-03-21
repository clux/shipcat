use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use merge::Merge;
use serde::de::DeserializeOwned;
use shipcat_definitions::{Config, Manifest, SimpleManifest, Region, Result};

use crate::manifest::{ManifestDefaults, ManifestOverrides, ManifestSource};

impl ManifestSource {
    pub fn load_manifest(service: &str, conf: &Config, reg: &Region) -> Result<Manifest> {
        let manifest = ManifestSource::load(service, conf, reg)?;
        manifest.build(&conf, &reg)
    }

    pub fn load_metadata(service: &str, conf: &Config, reg: &Region) -> Result<SimpleManifest> {
        let manifest = ManifestSource::load(service, conf, reg)?;
        manifest.build_simple(&reg)
    }

    fn load(service: &str, conf: &Config, reg: &Region) -> Result<Self> {
        let dir = Path::new(".").join("services").join(service);

        if !dir.exists() {
            bail!("Service folder {} does not exist", dir.display())
        }

        let global_defaults = ManifestDefaults::from_global(conf)?;
        let regional_defaults = ManifestDefaults::from_region(reg)?;
        let defaults = global_defaults.merge(regional_defaults);

        let source_path = dir.join("shipcat.yml");
        let source = ManifestSource::read_from(&source_path)?;
        let mut manifest = defaults.merge_source(source);

        let env_path = dir.join(format!("{}.yml", reg.environment.to_string()));
        if env_path.is_file() {
            let env = ManifestOverrides::read_from(&env_path)?;
            manifest = manifest.merge_overrides(env);
        }

        let region_path = dir.join(format!("{}.yml", reg.name));
        if region_path.is_file() {
            let region = ManifestOverrides::read_from(&region_path)?;
            manifest = manifest.merge_overrides(region);
        }

        Ok(manifest)
    }
}

impl ManifestDefaults {
    fn from_global(conf: &Config) -> Result<Self> {
        Ok(Self {
            chart: Option::Some(conf.defaults.chart.clone()),
            image_prefix: Option::Some(conf.defaults.imagePrefix.clone()),
            replica_count: Option::Some(conf.defaults.replicaCount),

            // TODO: Allow global env vars
            env: Default::default(),
        })
    }

    fn from_region(reg: &Region) -> Result<Self> {
        Ok(Self {
            env: reg.env.iter()
                .map(|(k, v)| (k.to_string(), v.into()))
                .collect(),

            // TODO: Allow more regional defaults
            chart: None,
            image_prefix: None,
            replica_count: None,
        })
    }
}

trait ManifestFile
where
    Self: Sized,
{
    fn read_from(path: &PathBuf) -> Result<Self>;
}

impl<T> ManifestFile for T
where
    T: DeserializeOwned,
{
    fn read_from(path: &PathBuf) -> Result<Self> {
        trace!("Reading manifest in {}", path.display());
        if !path.exists() {
            bail!("Manifest file {} does not exist", path.display())
        }
        let mut f = File::open(&path)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        if data.is_empty() {
            bail!("Manifest file {} is empty", path.display());
        }

        Ok(serde_yaml::from_str(&data)?)
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::{Path};

    use shipcat_definitions::{Config};
    use super::{ManifestSource};

    fn setup() {
        let pwd = env::current_dir().unwrap();
        let pth = fs::canonicalize(Path::new(&pwd).join("..").join("tests")).unwrap();
        std::env::set_current_dir(pth).unwrap();
    }

    #[test]
    fn load_fake_ask() {
        setup();

        let conf = Config::read().unwrap();
        let region = conf.get_region("dev-uk").unwrap();

        let manifest = ManifestSource::load_manifest("fake-ask", &conf, &region).unwrap();
        assert_eq!(manifest.name, "fake-ask".to_string());
    }

    #[test]
    fn load_fake_ask_metadata() {
        setup();

        let conf = Config::read().unwrap();
        let region = conf.get_region("dev-uk").unwrap();

        let manifest = ManifestSource::load_metadata("fake-ask", &conf, &region).unwrap();
        assert_eq!(manifest.name, "fake-ask".to_string());
        assert_eq!(manifest.version, Some("1.6.0".into()));
        assert_eq!(manifest.image, Some("quay.io/babylonhealth/fake-ask".into()));
    }
}
