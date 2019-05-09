use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use merge::Merge;
use serde::de::DeserializeOwned;
use shipcat_definitions::{Config, Manifest, Region, Result};
use walkdir::WalkDir;

use crate::manifest::{ManifestDefaults, ManifestOverrides, ManifestSource};
use super::{SimpleManifest, BaseManifest};
use super::authorization::{AuthorizationSource};
use super::util::{Build, Enabled};

impl ManifestSource {
    pub fn load_manifest(service: &str, conf: &Config, reg: &Region) -> Result<Manifest> {
        let manifest = ManifestSource::load_merged(service, conf, reg)?;
        manifest.build(&(conf.clone(), reg.clone()))
    }

    pub fn load_metadata(service: &str, conf: &Config, reg: &Region) -> Result<SimpleManifest> {
        let manifest = ManifestSource::load_merged(service, conf, reg)?;
        manifest.build_simple(&conf, &reg)
    }

    fn load_merged(service: &str, conf: &Config, reg: &Region) -> Result<Self> {
        let dir = Self::services_dir().join(service);

        if !dir.exists() {
            bail!("Service folder {} does not exist", dir.display())
        }

        let global_defaults = ManifestDefaults::from_global(conf)?;
        let regional_defaults = ManifestDefaults::from_region(reg)?;
        let defaults = global_defaults.merge(regional_defaults);

        let source_path = Self::services_dir().join(service).join("shipcat.yml");
        debug!("Loading service manifest from {:?}", source_path);
        let source = ManifestSource::read_from(&source_path)?;
        let mut manifest = defaults.merge_source(source);

        let env_path = dir.join(format!("{}.yml", reg.environment.to_string()));
        if env_path.is_file() {
            debug!("Loading service overrides from {:?}", env_path);
            let env = ManifestOverrides::read_from(&env_path)?;
            manifest = manifest.merge_overrides(env);
        }

        let region_path = dir.join(format!("{}.yml", reg.name));
        if region_path.is_file() {
            debug!("Loading service overrides from {:?}", region_path);
            let region = ManifestOverrides::read_from(&region_path)?;
            manifest = manifest.merge_overrides(region);
        }

        Ok(manifest)
    }

    fn all_names() -> Vec<String> {
        let mut res : Vec<_> = WalkDir::new(&ManifestSource::services_dir())
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir())
            .map(|e| {
                let mut cmps = e.path().components();
                cmps.next(); // .
                cmps.next(); // services
                let svccomp = cmps.next().unwrap();
                let svcname = svccomp.as_os_str().to_str().unwrap();
                svcname.to_string()
            })
            .collect();
        res.sort();
        res
    }

    pub fn all(conf: &Config) -> Result<Vec<BaseManifest>> {
        let mut all = vec![];
        for service in Self::all_names() {
            let source_path = Self::services_dir().join(service).join("shipcat.yml");
            debug!("Loading service manifest from {:?}", source_path);
            let source = ManifestSource::read_from(&source_path)?;
            let manifest = source.build_base(conf)?;
            all.push(manifest);
        }
        Ok(all)
    }

    pub fn available(conf: &Config, reg: &Region) -> Result<Vec<SimpleManifest>> {
        let mut available = vec![];
        for service in Self::all_names() {
            let manifest = Self::load_metadata(&service, conf, reg)?;
            if manifest.enabled && !manifest.external {
                available.push(manifest);
            }
        }
        Ok(available)
    }

    fn services_dir() -> PathBuf {
        Path::new(".").join("services")
    }
}

impl ManifestDefaults {

    fn from_global(conf: &Config) -> Result<Self> {
        let mut defs = Self::default();
        defs.chart = Option::Some(conf.defaults.chart.clone());
        defs.image_prefix = Option::Some(conf.defaults.imagePrefix.clone());
        defs.replica_count = Option::Some(conf.defaults.replicaCount.clone());

        Ok(defs)
    }

    fn from_region(reg: &Region) -> Result<Self> {
        let mut defs = Self::default();
        defs.env = reg.env.clone().into();
        if let Some(authz) = reg.defaults.kong.authorization.clone() {
            defs.kong.item.authorization = Enabled {
                enabled: None,
                item: AuthorizationSource {
                    allow_anonymous: Some(authz.allow_anonymous),
                    allowed_audiences: Some(authz.allowed_audiences),
                    allow_cookies: Some(authz.allow_cookies),
                    allow_invalid_tokens: Some(authz.allow_invalid_tokens),
                    required_scopes: Some(authz.required_scopes),
                },
            };
        }
        defs.kong.item.authorization.enabled = Some(reg.defaults.kong.authorizationEnabled);
        Ok(defs)
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
        assert_eq!(manifest.base.name, "fake-ask".to_string());
        assert_eq!(manifest.version, Some("1.6.0".into()));
        assert_eq!(manifest.image, Some("quay.io/babylonhealth/fake-ask".into()));
    }

    #[test]
    fn all() {
        setup();

        let conf = Config::read().unwrap();

        let all = ManifestSource::all(&conf).unwrap();

        let svc = &all[0];
        assert_eq!(svc.name, "external");

        let svc = &all[1];
        assert_eq!(svc.name, "fake-ask");

        let svc = &all[2];
        assert_eq!(svc.name, "fake-storage");

        let svc = &all[3];
        assert_eq!(svc.name, "out-of-region");
    }

    #[test]
    fn available() {
        setup();

        let conf = Config::read().unwrap();
        let region = conf.get_region("dev-uk").unwrap();

        let available = ManifestSource::available(&conf, &region).unwrap();
        assert_eq!(available.len(), 2);

        let manifest = &available[0];
        assert_eq!(manifest.base.name, "fake-ask".to_string());

        let manifest = &available[1];
        assert_eq!(manifest.base.name, "fake-storage".to_string());
    }
}
