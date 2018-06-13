#![allow(non_snake_case)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::prelude::*;

use semver::Version;
use serde_yaml;

use super::Result;
use super::structs::Kong;


#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct ManifestDefaults {
    /// Image prefix string
    pub imagePrefix: String,
    /// Chart to defer to
    pub chart: String,
    /// Default replication counts
    pub replicaCount: u32
}

/// Versioning Scheme used in region
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum VersionScheme {
    Semver,
    GitShaOrSemver,
}

impl Default for VersionScheme {
    fn default() -> Self {
        VersionScheme::GitShaOrSemver
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct RegionDefaults {
    /// Kubernetes namespace
    pub namespace: String,
    /// Environment (i.e: `dev` or `staging`)
    pub environment: String,
    /// Versioning scheme
    pub versions: VersionScheme,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct KongConfig {
    /// Base URL to use (e.g. uk.dev.babylontech.co.uk)
    #[serde(skip_serializing)]
    pub base_url: String,
    /// Configuration API URL (e.g. https://kong-admin-ops.dev.babylontech.co.uk)
    #[serde(skip_serializing)]
    pub config_url: String,
    /// Kong token expiration time (in seconds)
    pub kong_token_expiration: u32,
    pub oauth_provision_key: String,
    /// TCP logging options
    pub tcp_log: KongTcpLogConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anonymous_consumers: Option<KongAnonymousConsumers>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub consumers: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub internal_ips_whitelist: Vec<String>,
    #[serde(default, skip_serializing)]
    pub extra_apis: BTreeMap<String, Kong>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct KongAnonymousConsumers {
    pub anonymous: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct KongTcpLogConfig {
    pub enabled: bool,
    pub host: String,
    pub port: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Region {
    /// Region defaults
    pub defaults: RegionDefaults,
    /// Environment variables to inject
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Kong configuration for the region
    pub kong: KongConfig,
}


#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Team {
    /// Team name
    pub name: String,
}


/// Main manifest, serializable from shipcat.yml
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Global defaults
    pub defaults: ManifestDefaults,

    /// Allowed regions
    pub regions: BTreeMap<String, Region>,

    /// Teams
    pub teams: Vec<Team>,

    /// Shipcat version pin
    pub version: Version,
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
        }
        let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        if self.version > current {
            let url = "https://github.com/Babylonpartners/shipcat/releases";
            info!("Precompiled releasese available at {}", url);
            bail!("Your shipcat is out of date ({} < {})", current, self.version)
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

    /// Region validator
    pub fn region_defaults(&self, region: &str) -> Result<RegionDefaults> {
        if let Some(r) = self.regions.get(region) {
            Ok(r.defaults.clone())
        } else {
            bail!("You need to define your kube context '{}' in shipcat.conf first", region);
        }
    }
}
