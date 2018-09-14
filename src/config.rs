#![allow(non_snake_case)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::prelude::*;

use semver::Version;
use serde_yaml;

use super::vault::Vault;
use super::Result;
use super::structs::Kong;


#[derive(Serialize, Deserialize, Clone)]
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
    /// Version must be valid semver (no leading v)
    Semver,
    /// Version must be valid semver or a 40 character hex (git sha)
    GitShaOrSemver,
}

impl Default for VersionScheme {
    fn default() -> Self {
        VersionScheme::GitShaOrSemver
    }
}

/// Version validator
impl VersionScheme {
    pub fn verify(&self, ver: &str) -> Result<()> {
        use regex::Regex;
        let gitre = Regex::new(r"^[0-9a-f\-]{40}$").unwrap();
        match *self {
            VersionScheme::GitShaOrSemver => {
                if !gitre.is_match(&ver) && Version::parse(&ver).is_err() {
                    bail!("Illegal tag {} (floating tags cannot be rolled back please use 40 char git sha or semver)", ver);
                }
            },
            VersionScheme::Semver => {
                if Version::parse(&ver).is_err() {
                    bail!("Version {} is not a semver version in a region using semver versions", ver);
                }
            },
        };
        Ok(())
    }
}


#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Cluster {
    /// Url to the api server
    pub api: String,
    /// What regions this cluster control (perhaps not exclusively)
    pub regions: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct VaultConfig {
    /// Vault url up to and including port
    pub url: String,
    /// Root folder under secret
    pub folder: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct HostPort {
    /// Hostname || IP || FQDN
    pub host: String,
    /// Port
    pub port: u32,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct KafkaConfig {
    /// Broker urls
    pub brokers: Vec<String>,
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
    pub consumers: BTreeMap<String, KongOauthConsumer>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub internal_ips_whitelist: Vec<String>,
    #[serde(default, skip_serializing)]
    pub extra_apis: BTreeMap<String, Kong>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct KongAnonymousConsumers {
    pub anonymous: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct KongOauthConsumer {
    pub oauth_client_id: String,
    pub oauth_client_secret: String,
    pub username: String
}

impl KongConfig {
    pub fn secrets(&mut self, vault: &Vault, region: &str) -> Result<()> {
        for (svc, data) in &mut self.consumers {
            if data.oauth_client_id == "IN_VAULT" {
                let vkey = format!("{}/kong/consumers/{}_oauth_client_id", region, svc);
                data.oauth_client_id = vault.read(&vkey)?;
            }
            if data.oauth_client_secret == "IN_VAULT" {
                let vkey = format!("{}/kong/consumers/{}_oauth_client_secret", region, svc);
                data.oauth_client_secret = vault.read(&vkey)?;
            }
        }
        Ok(())
    }
    fn verify_secrets_exist(&self, vault: &Vault, region: &str) -> Result<()> {
        let mut expected = vec![];
        for (svc, data) in &self.consumers {
            if data.oauth_client_id == "IN_VAULT" {
                expected.push(format!("{}_oauth_client_id", svc));
            }
            if data.oauth_client_secret == "IN_VAULT" {
                expected.push(format!("{}_oauth_client_secret", svc));
            }
        }
        if expected.is_empty() {
            return Ok(()); // no point trying to cross reference
        }
        let secpth = format!("{}/kong/consumers", region);
        let found = vault.list(&secpth)?;
        debug!("Found kong secrets {:?} for {}", found, region);
        for v in expected {
            if !found.contains(&v) {
                bail!("Kong secret {} not found in {} vault", v, region);
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct KongTcpLogConfig {
    pub enabled: bool,
    pub host: String,
    pub port: String,
}

/// A region is an abstract kube context
///
/// Either it's a pure kubernetes context with a namespace and a cluster,
/// or it's an abstract concept with many associated real kubernetes contexts.
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Region {
    /// Kubernetes namespace
    pub namespace: String,
    /// Environment (e.g. `dev` or `staging`)
    pub environment: String,
    /// Versioning scheme
    pub versioningScheme: VersionScheme,

    /// Important base urls that can be templated in evars
    #[serde(default)]
    pub base_urls: BTreeMap<String, String>,

    /// Environment variables to inject
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Kong configuration for the region
    pub kong: KongConfig,
    /// List of Whitelisted IPs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ip_whitelist: Vec<String>,
    /// Kafka configuration for the region
    #[serde(default)]
    pub kafka: KafkaConfig,
    /// Vault configuration for the region
    pub vault: VaultConfig,
    /// List of locations the region serves
    #[serde(default)]
    pub locations: Vec<String>,
}

impl Region {
    pub fn secrets(&mut self, region: &str) -> Result<()> {
        let v = Vault::regional(&self.vault)?;
        self.kong.secrets(&v, region)?;
        Ok(())
    }
    pub fn verify_secrets_exist(&mut self, region: &str) -> Result<()> {
        let v = Vault::regional(&self.vault)?;
        debug!("Validating kong secrets for {}", region);
        self.kong.verify_secrets_exist(&v, region)?;
        Ok(())
    }
}


#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Location {
    /// Location name
    pub name: String,
}


#[derive(Serialize, Deserialize, Clone)]
pub struct Team {
    /// Team name
    pub name: String,
}


/// Main manifest, serializable from shipcat.yml
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Global defaults for the manifests
    pub defaults: ManifestDefaults,

    /// Cluster definitions
    pub clusters: BTreeMap<String, Cluster>,

    /// Context aliases, e.g. prod-uk-green -> prod-uk
    pub contextAliases: BTreeMap<String, String>,

    /// Region definitions
    pub regions: BTreeMap<String, Region>,

    /// Location definitions
    #[serde(default)]
    pub locations: BTreeMap<String, Location>,

    /// Team definitions
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

        let mut covered_regions = vec![];
        for (cname, clst) in &self.clusters {
            for r in &clst.regions {
                if !self.regions.contains_key(r) {
                    bail!("cluster {} defines undefined region {}", cname, r);
                }
                if covered_regions.contains(r) {
                    bail!("region {} is covered by multiple clusters", r);
                }
                covered_regions.push(r.to_string());
            }
        }

        for (k, v) in &self.contextAliases {
            // all contextAlias values must exist as defined regions
            if !self.regions.contains_key(v) {
                bail!("context alias {} points to undefined region {}", k, v);
            }
            // cannot alias something that exists!
            if self.regions.contains_key(k) {
                bail!("cannot self-alias region {}", k);
            }
        }

        for (r, data) in &self.regions {
            let region_parts : Vec<_> = r.split('-').collect();
            if region_parts.len() != 2 {
                bail!("invalid region {} of len {}", r, r.len());
            };
            if data.namespace == "" {
                bail!("Need to set `namespace` in {}", r);
            }
            if data.environment == "" {
                bail!("Need to set `environment` in {}", r)
            }
            if data.vault.url == "" {
                bail!("Need to set vault url for {}", r);
            }
            if data.vault.folder == "" {
                warn!("Need to set the vault folder {}", r);
            }
            for (_, v) in &data.base_urls {
                if v.ends_with('/') {
                    bail!("A base_url must not end with a slash");
                }
            }
            data.kong.verify()?;
        }
        let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        if self.version > current {
            let url = "https://github.com/Babylonpartners/shipcat/releases";
            info!("Precompiled releasese available at {}", url);
            bail!("Your shipcat is out of date ({} < {})", current, self.version)
        }
        Ok(())
    }

    /// Populate placeholder fields with secrets from vault
    pub fn secrets(&mut self, region: &str) -> Result<()> {
        if let Some(r) = self.regions.get_mut(region) {
            r.secrets(region)?;
        } else {
            bail!("Undefined region {}", region);
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

    /// Read a config in pwd and leave placeholders
    pub fn read() -> Result<Config> {
        let pwd = Path::new(".");
        let conf = Config::read_from(&pwd.to_path_buf())?;
        Ok(conf)
    }

    /// Retrieve region config from kube current context
    ///
    /// This should only be done once early on.
    /// The resulting name should be passed around internally as `region`.
    pub fn resolve_region(&self) -> Result<String> {
        use super::kube; // this should be the only user of current_context!
        let context = kube::current_context()?;
        if let Some(_) = self.regions.get(&context) {
            return Ok(context);
        }
        // otherwise search for an alias
        if let Some(alias) = self.contextAliases.get(&context) {
            return Ok(alias.to_string())
        }
        error!("Please use an existing kube context or add your current context to shipcat.conf");
        bail!("The current kube context ('{}') is not defined in shipcat.conf", context);
    }

    /// Region retriever safe alternative
    ///
    /// Alternative to the above `resolve_region`.
    /// Useful for small helper subcommands that do validation later.
    pub fn get_region(&self, ctx: &str) -> Result<(String, Region)> {
        if let Some(r) = self.regions.get(ctx) {
            return Ok((ctx.to_string(), r.clone()));
        } else if let Some(alias) = self.contextAliases.get(ctx) {
            return Ok((alias.to_string(), self.regions[&alias.to_string()].clone()));
            // missing region key should've been caught in verify above
        }
        bail!("You need to define your kube context '{}' in shipcat.conf regions first", ctx)
    }
}

impl KongConfig {
    fn verify(&self) -> Result<()> {
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::VersionScheme;
    #[test]
    fn version_validate_test() {
        let scheme = VersionScheme::default();
        assert!(scheme.verify("2.3.4").is_ok());
        assert!(scheme.verify("2.3.4-alpine").is_ok());
        assert!(scheme.verify("e7c1e5dd5de74b2b5da5eef76eb5bf12bdc2ac19").is_ok());
        assert!(scheme.verify("e7c1e5dd5de74b2b5da").is_err());
        assert!(scheme.verify("1.0").is_err());
        assert!(scheme.verify("v1.0.0").is_err());

        let svscheme = VersionScheme::Semver;
        assert!(svscheme.verify("2.3.4").is_ok());
        assert!(svscheme.verify("e7c1e5dd5de74b2b5da5eef76eb5bf12bdc2ac19").is_err());
    }
}
