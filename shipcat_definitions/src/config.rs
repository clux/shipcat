#![allow(non_snake_case)]

use semver::Version;
use std::collections::{BTreeMap, BTreeSet};


use crate::teams;
#[allow(unused_imports)] use std::path::{Path, PathBuf};

#[allow(unused_imports)] use super::{Error, Result};
use crate::{
    region::{Environment, Region},
    states::ConfigState,
};

// ----------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "filesystem", serde(deny_unknown_fields))]
pub struct ManifestDefaults {
    /// Image prefix string
    pub imagePrefix: String,
    /// Chart to defer to
    pub chart: String,
    /// Default replication counts
    pub replicaCount: u32,
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

/// Kubernetes cluster information
#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "filesystem", serde(deny_unknown_fields))]
pub struct Cluster {
    /// Name of the cluster
    pub name: String,
    /// Url to the Kubernetes api server
    pub api: String,
    /// Teleport url to use with tsh login
    #[serde(default)]
    pub teleport: Option<String>,
    /// What regions this cluster control (perhaps not exclusively)
    pub regions: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "filesystem", serde(deny_unknown_fields))]
pub struct Location {
    /// Location name
    pub name: String,

    /// Name of global region
    pub global_region: String,

    /// Name of local region
    pub local_region: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "filesystem", serde(deny_unknown_fields))]
pub struct GithubParameters {
    /// Organisation name
    pub organisation: String,
}


#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "filesystem", serde(deny_unknown_fields))]
pub struct SlackParameters {
    /// Team name (T...)
    pub team: String,
}


// ----------------------------------------------------------------------------------


/// Main manifest, serializable from shipcat.conf
#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "filesystem", serde(deny_unknown_fields))]
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

    /// Slack parameters
    pub slack: SlackParameters,

    /// Gihub parameters
    pub github: GithubParameters,

    /// Allowed labels
    #[serde(default)]
    pub allowedLabels: Vec<String>,

    #[serde(default)]
    pub allowedCustomMetadata: BTreeSet<String>,

    /// Shipcat version pins
    pub versions: BTreeMap<Environment, Version>,

    /// Owners of services, squads, tribes
    ///
    /// Populated from teams.yml
    #[serde(default)]
    pub owners: teams::Owners,

    // Internal state of the config
    #[serde(default, skip_serializing, skip_deserializing)]
    state: ConfigState,
}

impl Config {
    pub fn verify(&self) -> Result<()> {
        let defs = &self.defaults;
        // verify default chart exists
        if cfg!(feature = "filesystem") {
            let chart = Path::new(".").join("charts").join(&defs.chart).join("Chart.yaml");
            if !chart.is_file() {
                bail!("Default chart {} does not exist", self.defaults.chart);
            }
        }
        if defs.imagePrefix.ends_with('/') {
            bail!("image prefix must not end with a slash");
        }

        for (cname, clst) in &self.clusters {
            if cname != &clst.name {
                bail!(
                    "clust '{}' must have a '.name' equal to its key in clusters",
                    cname
                );
            }
            // can't actually verify this in a smaller manifest..
            #[cfg(feature = "filesystem")]
            for r in &clst.regions {
                if !self.has_region(r) && self.state == ConfigState::File {
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
            if r.cluster == "" {
                bail!("Need to set the serving `cluster` of {}", r.name);
            }
            if !self.clusters.keys().any(|c| c == &r.cluster) {
                bail!("Region {} served by missing cluster '{}'", r.name, r.cluster);
            }
            r.vault.verify(&r.name)?;
            for v in r.base_urls.values() {
                if v.ends_with('/') {
                    bail!("A base_url must not end with a slash");
                }
            }
            if let Some(kong) = &r.kong {
                kong.verify()?;
                if used_kong_urls.contains(&kong.config_url) {
                    bail!("Cannot reuse kong config urls for {} across regions", r.name);
                }
                used_kong_urls.push(kong.config_url.clone());
            }
        }
        Ok(())
    }

    #[cfg(feature = "filesystem")]
    pub fn verify_version_pin(&self, env: &Environment) -> Result<()> {
        let pin = self.get_appropriate_version_pin(env)?;
        debug!("Verifying version pin: {} for {}", pin, env.to_string());
        Config::bail_on_version_older_than(&pin)
    }

    #[cfg(feature = "filesystem")]
    pub fn get_appropriate_version_pin(&self, env: &Environment) -> Result<Version> {
        let pin = self.versions.get(&env).unwrap_or_else(|| {
            // NB: this fails in unpinned envs - still doing verification
            debug!("No version pin for environment {:?} - assuming maximum", env);
            self.versions
                .values()
                .max()
                .expect("a version pin exists in shipcat.conf")
        });
        Ok(pin.clone())
    }

    #[cfg(feature = "filesystem")]
    pub fn bail_on_version_older_than(pin: &Version) -> Result<()> {
        let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        if *pin > current {
            let releasesurl = "https://github.com/babylonhealth/shipcat/releases";
            let brewurl = "https://github.com/babylonhealth/homebrew-babylon";
            let install = format!(
                "Precompiled releases for mac/linux from {} installeable via:
\tshipcat self-upgrade
",
                releasesurl
            );

            // Babylon convenience
            let brewinstall = format!(
                "Homebrew managed mac releases from {} installeable via:
\tbrew update && brew upgrade shipcat
",
                brewurl
            ); // brew update technically only necessary for ssh based taps

            bail!(
                "Your shipcat is out of date ({} < {})\n{}\n{}",
                current,
                pin,
                install,
                brewinstall
            )
        }
        Ok(())
    }

    /// Print Config to stdout
    pub fn print(&self) -> Result<()> {
        println!("{}", serde_yaml::to_string(self)?);
        Ok(())
    }

    /// Helper for small utils that don't need the full struct
    pub fn list_regions(&self) -> Vec<String> {
        self.regions.iter().map(|r| r.name.clone()).collect()
    }

    /// Fill secrets from vault on a Base config for a known to exist region
    ///
    /// This will use the HTTP api of Vault using the configuration parameters.
    #[cfg(feature = "filesystem")]
    fn secrets(&mut self, region: &str) -> Result<()> {
        assert_eq!(self.state, ConfigState::Base);
        assert_eq!(self.regions.len(), 1);
        self.state = ConfigState::Filtered;
        if let Some(idx) = self.regions.iter().position(|r| r.name == region) {
            self.regions[idx].secrets()?;
        } else {
            bail!("Region {} does not exist in the config", region)
        }
        Ok(())
    }

    /// Work out the cluster from the kube config
    ///
    /// Can be done unambiguously when cluster name is specified,
    /// Otherwise we will find the first candidate cluster serving this context
    /// and bail if there's more than one.
    pub fn resolve_cluster(&self, ctx: &str, cluster: Option<String>) -> Result<(Cluster, Region)> {
        let reg = self.get_region(ctx)?;

        // 1. `get -r dev-uk -c kops-uk`
        // Most precise - just get what asked for.
        // region must exist because `ctx` is either a region or a contextAlias
        if let Some(c) = cluster {
            if let Some(c) = self.clusters.get(&c) {
                return Ok((c.clone(), reg));
            } else {
                bail!("Specified cluster '{}' does not exist in shipcat.conf", c);
            }
        }

        // 2. `get -r preprodca-blue`
        // Infer cluster from the one serving it (if only one)
        let candidates = self
            .clusters
            .values()
            .cloned()
            .filter(|v| v.regions.contains(&reg.name))
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            bail!(
                "Ambiguous context {} served by more than one cluster - please specify -c cluster",
                ctx
            );
        }
        Ok((candidates[0].clone(), reg))
    }

    pub fn has_secrets(&self) -> bool {
        self.state == ConfigState::Filtered
    }

    /// Retrieve region name using either a region name, or a context as a fallback
    ///
    /// This returns a a valid key in `self.regions` if Some.
    fn resolve_context(&self, ctx: String) -> Option<String> {
        if self.has_region(&ctx) {
            Some(ctx)
        }
        // otherwise search for an alias
        else {
            // NB: existing alias is guaranteed to have a corresponding region by verify
            self.contextAliases.get(&ctx).map(|a| a.to_string())
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
        if let Some(region) = self.resolve_context(ctx.to_string()) {
            return Ok(self.regions.iter().find(|r| r.name == region).unwrap().clone());
        }
        bail!(
            "You need to define your kube context '{}' in shipcat.conf regions first",
            ctx
        )
    }

    /// Region exposer (needed in a few special cases, raftcat, crd reconcile)
    pub fn get_regions(&self) -> Vec<Region> {
        self.regions.clone()
    }

    /// Find the Cluster struct that owns this Region
    pub fn find_owning_cluster(&self, region: &Region) -> Option<Cluster> {
        for c in self.clusters.values() {
            if c.regions.iter().any(|r| r == &region.name) {
                if c.name != region.cluster {
                    warn!("Inactive cluster: {} for {}", c.name, region.name);
                } else {
                    return Some(c.clone());
                }
            }
        }
        None
    }
}


/// Simplified config with version information only
///
/// The part of shipcat.conf you never get to break the format of.
#[derive(Deserialize)]
pub struct ConfigFallback {
    pub versions: BTreeMap<Environment, Version>,
}

impl ConfigFallback {
    /// Read the fallback version of the Config to decide if upgrade needed
    fn read() -> Result<ConfigFallback> {
        use std::fs;
        let pwd = Path::new(".");
        let mpath = pwd.join("shipcat.conf");
        let data = fs::read_to_string(&mpath)?;
        let vc: ConfigFallback = serde_yaml::from_str(&data)?;
        Ok(vc)
    }

    /// Safety path when shipcat.conf is unreadeable due to schema changes
    ///
    /// Allows main to identify an upgrade path without a working Config
    pub fn find_upgradeable_version() -> Result<Option<Version>> {
        // We should always be able to read fallback ConfigFallback:
        let fb = match Self::read() {
            Ok(cfg) => cfg,
            Err(e) => {
                error!("shipcat.conf versions unreadable - manifests out of date?");
                bail!("shipcat.conf could not be read even in fallback mode: {}", e);
            }
        };
        // If only fallback ok, a schema change probably caused it.
        // Check if our shipcat is out of date:
        match fb.versions.values().min() {
            None => bail!("Failed to understand version pin in shipcat.conf - needs at least one pin"),
            Some(lowest) => {
                let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
                if *lowest > current {
                    return Ok(Some(lowest.to_owned()));
                }
                // otherwise.. we are up to date - unfortunately.
            }
        }
        Ok(None)
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
    pub fn new(state: ConfigState, context: &str) -> Result<(Config, Region)> {
        let mut conf = Self::read()?;
        let region = if let Some(r) = conf.resolve_context(context.to_string()) {
            r
        } else {
            error!("Please use an existing kube context or add your current context to shipcat.conf");
            bail!(
                "The current kube context ('{}') is not defined in shipcat.conf",
                context
            );
        };

        if state == ConfigState::Filtered || state == ConfigState::Base {
            conf.remove_redundant_regions(&region)?;
        } else if state != ConfigState::UnionisedBase {
            bail!("Config::new only supports Filtered, Base and UnionisedBase types");
        }

        if state == ConfigState::Filtered {
            conf.secrets(&region)?;
        }
        let reg = conf.get_region(&region)?;
        Ok((conf, reg))
    }

    /// Read a config file in an arbitrary path
    fn read_from(pwd: &PathBuf) -> Result<Config> {
        use std::fs;
        let mpath = pwd.join("shipcat.conf");
        trace!("Using config in {}", mpath.display());
        if !mpath.exists() {
            bail!("Config file {} does not exist", mpath.display())
        }
        let data = fs::read_to_string(&mpath)?;
        let res = serde_yaml::from_str(&data)?;
        Ok(res)
    }

    /// Read a config in pwd and leave placeholders
    pub fn read() -> Result<Config> {
        let pwd = Path::new(".");
        let mut conf = Config::read_from(&pwd.to_path_buf())?;
        conf.owners = teams::Owners::read()?;
        Ok(conf)
    }

    pub fn has_all_regions(&self) -> bool {
        self.state == ConfigState::File
    }

    /// Region retriever for global reducers
    ///
    /// Assumes you have not filtered a config in main accidentally.
    pub fn get_region_unchecked(&self, region: &str) -> Option<&Region> {
        assert!(self.has_all_regions());
        self.regions.iter().find(|r| r.name == region)
    }

    /// Filter a file based config for a known to exist region
    fn remove_redundant_regions(&mut self, region: &str) -> Result<()> {
        assert_eq!(self.state, ConfigState::File);
        assert!(self.has_region(region));
        let r = region.to_string();
        // filter out cluster and aliases as well so we don't have to special case verify
        self.clusters = self
            .clusters
            .clone()
            .into_iter()
            .filter(|(_, c)| c.regions.contains(&r))
            .collect();
        self.contextAliases = self
            .contextAliases
            .clone()
            .into_iter()
            .filter(|(_, v)| v == region)
            .collect();
        self.regions = self
            .regions
            .clone()
            .into_iter()
            .filter(|r| r.name == region)
            .collect();
        self.state = ConfigState::Base;
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use crate::region::VersionScheme;
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
        assert!(svscheme
            .verify("e7c1e5dd5de74b2b5da5eef76eb5bf12bdc2ac19")
            .is_err());
    }
}
