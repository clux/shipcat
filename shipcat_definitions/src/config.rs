#![allow(non_snake_case)]

use std::collections::BTreeMap;
use semver::Version;

#[allow(unused_imports)]
use std::path::{Path, PathBuf};
use structs::SlackChannel;

use super::Vault;
#[allow(unused_imports)]
use super::{Result, Error};
use super::structs::{Kong, Contact};
use states::ConfigType;

// ----------------------------------------------------------------------------------


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
// Allow smaller base configs
impl Default for ManifestDefaults {
    fn default() -> Self {
        ManifestDefaults {
            chart: "base".into(),
            replicaCount: 1,
            imagePrefix: "".into(),
        }
    }
}

/// Versioning Scheme used in region
///
/// This is valdiated strictly using `shipcat validate` when versions are found in manifests.
/// Otherwise, it's validated on upgrade time (via `shipcat apply`) when it's passed.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum VersionScheme {
    /// Version must be valid semver (no leading v)
    ///
    /// This is the assumed default for regions that lock versions in manifests.
    Semver,
    /// Version must be valid semver or a 40 character hex (git sha)
    ///
    /// This can be used for rolling environments that does not lock versions in manifests.
    GitShaOrSemver,
}

impl Default for VersionScheme {
    fn default() -> VersionScheme {
        VersionScheme::Semver
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


/// Kubernetes cluster information
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Cluster {
    /// Name of the cluster
    pub name: String,
    /// Url to the Kubernetes api server
    pub api: String,
    /// What regions this cluster control (perhaps not exclusively)
    pub regions: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Team {
    /// Team name
    pub name: String,
    /// Code owners for this team
    #[serde(default)]
    pub owners: Vec<Contact>,
    #[serde(default)]
    /// Default support channel - human interaction
    pub support: Option<SlackChannel>,
    /// Default notifications channel - automated messages
    #[serde(default)]
    pub notifications: Option<SlackChannel>,
}


/// Vault configuration for a region
#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(Default))]
#[serde(deny_unknown_fields)]
pub struct VaultConfig {
    /// Vault url up to and including port
    pub url: String,
    /// Root folder under secret/
    ///
    /// Typically, the name of the region to disambiguate.
    pub folder: String,
}

//#[derive(Serialize, Deserialize, Clone, Default)]
//#[serde(deny_unknown_fields)]
//pub struct HostPort {
//    /// Hostname || IP || FQDN
//    pub host: String,
//    /// Port
//    pub port: u32,
//}

/// Kafka configuration for a region
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct KafkaConfig {
    /// Broker urls in "hostname:port" format.
    ///
    /// These are injected in to the manifest.kafka struct if it's set.
    pub brokers: Vec<String>,
}

// ----------------------------------------------------------------------------------

/// Kong configuration for a region
#[derive(Serialize, Deserialize, Clone, Default)] // TODO: better Default impl
#[serde(deny_unknown_fields)]
pub struct KongConfig {
    /// Base URL to use (e.g. uk.dev.babylontech.co.uk)
    pub base_url: String,
    /// Configuration API URL (e.g. https://kong-admin-ops.dev.babylontech.co.uk)
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
    fn secrets(&mut self, vault: &Vault, region: &str) -> Result<()> {
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
        if self.oauth_provision_key == "IN_VAULT" {
            let vkey = format!("{}/kong/oauth_provision_key", region);
            self.oauth_provision_key = vault.read(&vkey)?;
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

impl KongConfig {
    fn verify(&self) -> Result<()> {
        Ok(())
    }
}

// ----------------------------------------------------------------------------------

/// A region is an abstract kube context
///
/// Either it's a pure kubernetes context with a namespace and a cluster,
/// or it's an abstract concept with many associated real kubernetes contexts.
#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(Default))]
#[serde(deny_unknown_fields)]
pub struct Region {
    /// Name of region
    pub name: String,
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
    #[serde(default)]
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
    // Internal secret populator for Config::new
    fn secrets(&mut self) -> Result<()> {
        let v = Vault::regional(&self.vault)?;
        self.kong.secrets(&v, &self.name)?;
        Ok(())
    }

    // Entry point for region verifier
    pub fn verify_secrets_exist(&mut self) -> Result<()> {
        let v = Vault::regional(&self.vault)?;
        debug!("Validating kong secrets for {}", self.name);
        self.kong.verify_secrets_exist(&v, &self.name)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Location {
    /// Location name
    pub name: String,
}

// ----------------------------------------------------------------------------------


/// Main manifest, serializable from shipcat.yml
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Global defaults for the manifests
    #[serde(default)]
    pub defaults: ManifestDefaults,

    /// Cluster definitions
    pub clusters: BTreeMap<String, Cluster>,

    /// Context aliases, e.g. prod-uk-green -> prod-uk
    #[serde(default)]
    pub contextAliases: BTreeMap<String, String>,

    /// Region definitions
    ///
    /// Not public because access regions may or may not have secrets filled in.
    /// This makes sure we don't start using the wrong one.
    regions: Vec<Region>,

    /// Location definitions
    #[serde(default)]
    pub locations: BTreeMap<String, Location>,

    /// Team definitions
    pub teams: Vec<Team>,

    /// Shipcat version pin
    pub version: Version,

    // Internal state of the config
    #[serde(default, skip_serializing, skip_deserializing)]
    kind: ConfigType,
}

impl Config {
    pub fn verify(&self) -> Result<()> {
        let defs = &self.defaults;
        // verify default chart exists
        if cfg!(feature = "filesystem") {
            let chart = Path::new(".").join("charts").join(&defs.chart).join("Chart.yaml");
            if ! chart.is_file() {
                bail!("Default chart {} does not exist", self.defaults.chart);
            }
        }
        if defs.imagePrefix.ends_with('/') {
            bail!("image prefix must not end with a slash");
        }

        for (cname, clst) in &self.clusters {
            if cname != &clst.name {
                bail!("clust '{}' must have a '.name' equal to its key in clusters", cname);
            }
            // can't actually verify this in a smaller manifest..
            #[cfg(feature = "filesystem")]
            for r in &clst.regions {
                if !self.has_region(r) && self.kind == ConfigType::File {
                    bail!("cluster {} defines undefined region {}", cname, r);
                }
            }
        }

        for (k, v) in &self.contextAliases {
            // all contextAlias values must exist as defined regions
            if !self.has_region(v) {
                bail!("context alias {} points to undefined region {}", k, v);
            }
            // cannot alias something that exists!
            if self.has_region(k) {
                bail!("cannot self-alias region {}", k);
            }
        }

        let mut used_kong_urls = vec![];
        for r in &self.regions {
            if r.namespace == "" {
                bail!("Need to set `namespace` in {}", r.name);
            }
            if r.environment == "" {
                bail!("Need to set `environment` in {}", r.name)
            }
            if r.vault.url == "" {
                bail!("Need to set vault url for {}", r.name);
            }
            if r.vault.folder == "" {
                warn!("Need to set the vault folder {}", r.name);
            }
            for (_, v) in &r.base_urls {
                if v.ends_with('/') {
                    bail!("A base_url must not end with a slash");
                }
            }
            r.kong.verify()?;
            if used_kong_urls.contains(&r.kong.config_url) {
                bail!("Cannot reuse kong config urls for {} across regions", r.name);
            }
            used_kong_urls.push(r.kong.config_url.clone());
        }
        for t in &self.teams {
            for o in &t.owners {
                o.verify()?; // not very strict
                // verify optionals filled in for owners:
                if o.github.is_none() {
                    bail!("Every owner must have a github id attached");
                }
            }
            if t.support.is_none() {
                bail!("Every team must have a default support channel declared");
            }
            if t.notifications.is_none() {
                bail!("Every team must have a default notifications channel declared");
            }
        }
        Config::verify_version(&self.version)?;

        Ok(())
    }

    fn verify_version(ver: &Version) -> Result<()> {
        let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        if ver > &current {
            let url = "https://github.com/Babylonpartners/shipcat/releases";
            let brewurl = "https://github.com/Babylonpartners/homebrew-babylon";
            let s1 = format!("Precompiled releases available at {}", url);
            let s2 = format!("Homebrew releases available via {}", brewurl);

            bail!("Your shipcat is out of date ({} < {})\n\t{}\n\t{}", current, ver, s1, s2)
        }
        Ok(())
    }



    /// Print Config to stdout
    pub fn print(&self) -> Result<()> {
        println!("{}", serde_yaml::to_string(self)?);
        Ok(())
    }

    /// Helper for list::regions
    pub fn list_regions(&self) -> Vec<String> {
        self.regions.iter().map(|r| r.name.clone()).collect()
    }

    /// Complete a Base config for a know to exist region
    fn complete(&mut self, region: &str) -> Result<()> {
        assert_eq!(self.kind, ConfigType::Base);
        assert_eq!(self.regions.len(), 1);
        self.kind = ConfigType::Completed;
        if let Some(idx) = self.regions.iter().position(|r| r.name == region) {
            self.regions.get_mut(idx).unwrap().secrets()?;
        } else {
            bail!("Region {} does not exist in the config", region)
        }
        Ok(())
    }


    /// Work out the cluster from the kube config
    ///
    /// Can be done unambiguously in two cases:
    /// - cluster.name === kube context name
    /// - region.name === kube context name && region is served by one cluster only
    pub fn resolve_cluster(&self, ctx: &str) -> Result<(Cluster, Region)> {
        let reg = self.get_region(ctx)?;

        // 1: `cluster.name == context` (dump apps cluster with dedicated context)
        // e.g. `preproduk-blue` cluster serves `preprod-uk` region
        if let Some(c) = self.clusters.get(ctx) {
            return Ok((c.clone(), reg));
        }
        // 2: `region.name == context` (big cluster with many namespaces and regions)
        // e.g. `kops-global` cluster serving `dev-global` + `staging-global` regions
        let candidates = self.clusters.values().cloned().filter(|v| {
            v.regions.contains(&reg.name)
        }).collect::<Vec<_>>();
        if candidates.len() != 1 {
            bail!("Ambiguous context {} must be served by exactly one cluster", ctx);
        }
        Ok((candidates[0].clone(), reg))
    }

    pub fn has_secrets(&self) -> bool {
        self.kind == ConfigType::Completed
    }

    /// Retrieve region name using either a region name, or a context as a fallback
    ///
    /// This returns a a valid key in `self.regions` if Some.
    fn resolve_context(&self, context: &str) -> Option<String> {
        let ctx = context.to_string();
        if self.has_region(&ctx) {
            Some(ctx)
        }
        // otherwise search for an alias
        else if let Some(alias) = self.contextAliases.get(&ctx) {
            // NB: alias is guaranteed to have a corresponding region by verify
            Some(alias.to_string())
        } else {
            None
        }
    }

    fn has_region(&self, region: &str) -> bool {
        self.regions.iter().any(|r| r.name == region)
    }

    /// Region retriever interface
    ///
    /// This retieves the region after calling resolve_context.
    /// Useful for small helper subcommands that do validation later.
    pub fn get_region(&self, ctx: &str) -> Result<Region> {
        if let Some(region) = self.resolve_context(ctx) {
            return Ok(self.regions.iter().find(|r| r.name == region).unwrap().clone())
        }
        bail!("You need to define your kube context '{}' in shipcat.conf regions first", ctx)
    }
}

/// Filesystem accessors for Config
///
/// These must live in here because they use private methods herein.
#[cfg(feature = "filesystem")]
impl Config {
    /// Main constructor for CLI
    ///
    /// Pass this a region request via argument or a current context
    pub fn new(kind: ConfigType, context: &str) -> Result<(Config, Region)> {
        let mut conf = Config::read()?;
        let region = if let Some(r) = conf.resolve_context(context) {
            r
        } else {
            error!("Please use an existing kube context or add your current context to shipcat.conf");
            bail!("The current kube context ('{}') is not defined in shipcat.conf", context);
        };

        if kind == ConfigType::Completed || kind == ConfigType::Base {
            conf.remove_redundant_regions(&region)?;
        } else {
            bail!("Config::new only supports Completed or Base type");
        }

        if kind == ConfigType::Completed {
            conf.complete(&region)?;
        }
        let reg = conf.get_region(&region)?;
        Ok((conf, reg))
    }

    /// Read a config file in an arbitrary path
    fn read_from(pwd: &PathBuf) -> Result<Config> {
        use std::fs::File;
        use std::io::prelude::*;
        use semver::Version;
        let mpath = pwd.join("shipcat.conf");
        trace!("Using config in {}", mpath.display());
        if !mpath.exists() {
            bail!("Config file {} does not exist", mpath.display())
        }
        let mut f = File::open(&mpath)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        let res = serde_yaml::from_str(&data);
        match res {
            Err(e) => {
                // failed to parse the config common causes:

                // 1. version change caused their config to be out of date
                use regex::Regex;
                // manually check version against running
                let ver_re = Regex::new(r"^version:\s*(?P<version>.+)$").unwrap();
                for l in data.lines() {
                    if let Some(caps) = ver_re.captures(l) {
                        debug!("got version from raw data: {:?}", caps);
                        // take precedence over config parse error if we can get a version in config
                        if let Ok(expected) = Version::parse(&caps["version"]) {
                            let res2 = Config::verify_version(&expected);
                            if let Err(e2) = res2 {
                                return Err(Error::from(e).chain_err(|| e2));
                            }
                        }
                    }
                }
                // 2. manifests out of date, but shipcat up to date (can happen with brew)
                // TODO: git check in SHIPCAT_MANIFEST_DIR ?
                warn!("Invalid shipcat.conf - either genuine error, or your manifests dir is out of date locally");

                // 3. genuine mistake in shipcat.conf additions/removals
                return Err(e.into()) // propagate normal error
            }
            Ok(d) => Ok(d)
        }
    }

    /// Read a config in pwd and leave placeholders
    pub fn read() -> Result<Config> {
        let pwd = Path::new(".");
        let conf = Config::read_from(&pwd.to_path_buf())?;
        Ok(conf)
    }

    pub fn has_all_regions(&self) -> bool {
        self.kind == ConfigType::File
    }

    /// Filter a file based config for a known to exist region
    fn remove_redundant_regions(&mut self, region: &str) -> Result<()> {
        assert_eq!(self.kind, ConfigType::File);
        assert!(self.has_region(region));
        let r = region.to_string();
        // filter out cluster and aliases as well so we don't have to special case verify
        self.clusters = self.clusters.clone().into_iter().filter(|(_, c)| c.regions.contains(&r)).collect();
        self.contextAliases = self.contextAliases.clone().into_iter().filter(|(_, v)| v == region).collect();
        self.regions = self.regions.clone().into_iter().filter(|r| r.name == region).collect();
        self.kind = ConfigType::Base;
        Ok(())
    }
}



#[cfg(test)]
mod tests {
    use super::VersionScheme;
    #[test]
    fn version_validate_test() {
        let scheme = VersionScheme::GitShaOrSemver;
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
