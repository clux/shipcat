#![allow(non_snake_case)]

use semver::Version;
use std::collections::BTreeMap;


#[allow(unused_imports)]
use std::path::{Path, PathBuf};
use crate::structs::SlackChannel;


#[allow(unused_imports)]
use super::{Result, Error};
use super::structs::{Contact};
use crate::states::ConfigType;
use crate::region::Region;

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

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Location {
    /// Location name
    pub name: String,

    /// Name of global region
    pub global_region: String,

    /// Name of local region
    pub local_region: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct GithubParameters {
    /// Location name
    pub organisation: String,
}


#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct SlackParameters {
    /// Location name
    pub team: String,
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

    /// Slack parameters
    pub slack: SlackParameters,

    /// Gihub parameters
    pub github: GithubParameters,

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
            for v in r.base_urls.values() {
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

    /// Fill secrets from vault on a Base config for a known to exist region
    ///
    /// This will use the HTTP api of Vault using the configuration parameters.
    #[cfg(feature = "filesystem")]
    fn secrets(&mut self, region: &str) -> Result<()> {
        assert_eq!(self.kind, ConfigType::Base);
        assert_eq!(self.regions.len(), 1);
        self.kind = ConfigType::Filtered;
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
        let candidates = self.clusters.values().cloned().filter(|v| {
            v.regions.contains(&reg.name)
        }).collect::<Vec<_>>();
        if candidates.len() != 1 {
            bail!("Ambiguous context {} served by more than one cluster - please specify -c cluster", ctx);
        }
        Ok((candidates[0].clone(), reg))
    }

    pub fn has_secrets(&self) -> bool {
        self.kind == ConfigType::Filtered
    }

    /// Retrieve region name using either a region name, or a context as a fallback
    ///
    /// This returns a a valid key in `self.regions` if Some.
    fn resolve_context(&self, context: String) -> Option<String> {
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
        if let Some(region) = self.resolve_context(ctx.to_string()) {
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
        let region = if let Some(r) = conf.resolve_context(context.to_string()) {
            r
        } else {
            error!("Please use an existing kube context or add your current context to shipcat.conf");
            bail!("The current kube context ('{}') is not defined in shipcat.conf", context);
        };

        if kind == ConfigType::Filtered || kind == ConfigType::Base {
            conf.remove_redundant_regions(&region)?;
        } else if kind != ConfigType::UnionisedBase {
            bail!("Config::new only supports Filtered, Base and UnionisedBase types");
        }

        if kind == ConfigType::Filtered {
            conf.secrets(&region)?;
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
        assert!(svscheme.verify("e7c1e5dd5de74b2b5da5eef76eb5bf12bdc2ac19").is_err());
    }
}
