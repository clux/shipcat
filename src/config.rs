#![allow(non_snake_case)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::prelude::*;

use serde_yaml;

use super::Result;
//use super::vault::Vault;


#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ManifestDefaults {
    /// Image prefix string
    pub imagePrefix: String,
    /// Chart to defer to
    pub chart: String,
    /// Default replication counts
    pub replicaCount: u32
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct RegionDefaults {
    /// Kubernetes namespace
    pub namespace: String,
    /// Docker image floating tag
    pub version: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Region {
    /// Region defaults
    pub defaults: RegionDefaults,
    /// Environment variables to inject
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
}


#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Team {
    /// Team name
    pub name: String,
}


/// Main manifest, serializable from shipcat.yml
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Config {
    /// Global defaults
    pub defaults: ManifestDefaults,

    /// Allowed regions
    pub regions: BTreeMap<String, Region>,

    /// Teams
    pub teams: Vec<Team>,
}

impl Config {
    pub fn verify(&self) -> Result<()> {
        let defs = &self.defaults;
        // verify default chart exists
        let chart = Path::new(".").join("charts").join(&defs.chart).join("Chart.yaml");
        if ! chart.is_file() {
            bail!("Default chart {} does not exist", self.defaults.chart);
        }
        if defs.imagePrefix == "" || defs.imagePrefix.ends_with('/') {
            bail!("image prefix must be non-empty and not end with a slash");
        }

        for (r, data) in &self.regions {
            let region_parts : Vec<_> = r.split('-').collect();
            if region_parts.len() != 2 {
                bail!("invalid region {} of len {}", r, r.len());
            };
            let rdefs = &data.defaults;
            if rdefs.namespace == "" {
                bail!("Default namespace cannot be empty");
            }
            if rdefs.version == "" {
                bail!("Default floating tag must be set");
            }
        }

        Ok(())
    }

    /// Read a config file in an arbitrary path
    fn read_from(pwd: &PathBuf) -> Result<Config> {
        let mpath = pwd.join("shipcat.conf");
        trace!("Using config in {}", mpath.display());
        if !mpath.exists() {
            bail!("Config file {} does not exist", mpath.display())
        }
        let mut f = File::open(&mpath)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        Ok(serde_yaml::from_str(&data)?)
    }

    /// Read a config in pwd
    pub fn read() -> Result<Config> {
        let pwd = Path::new(".");
        let conf = Config::read_from(&pwd.to_path_buf())?;
        Ok(conf)
    }
}
