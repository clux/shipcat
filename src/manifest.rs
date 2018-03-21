#![allow(non_snake_case)]

use serde_yaml;
use walkdir::WalkDir;
use regex::Regex;

use std::io::prelude::*;
use std::fs::File;
use std::path::{PathBuf, Path};
use std::collections::BTreeMap;

use super::{Result, Config};
use super::vault::Vault;

// All structs come from the structs directory
use super::structs::traits::Verify;
use super::structs::{HealthCheck, ConfigMap};
use super::structs::{InitContainer, Resources, HostAlias};
use super::structs::volume::{Volume, VolumeMount};
use super::structs::{Metadata, DataHandling, VaultOpts, Jaeger, Dependency};
use super::structs::prometheus::{Prometheus, Dashboard};
use super::structs::{CronJob};


/// Main manifest, serializable from shipcat.yml
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Manifest {
    /// Name of the service
    #[serde(default)]
    pub name: String,

    /// Wheter to ignore this service
    #[serde(default, skip_serializing)]
    pub disabled: bool,
    /// Wheter the service is externally managed
    #[serde(default)]
    pub external: bool,

    /// Optional image name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Optional version/tag of docker image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Optional image command (if not using the default docker command)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,

    /// Canonical data sources like repo, docs, team names
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    /// Data sources and handling strategies
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dataHandling: Vec<DataHandling>,

    /// Jaeger options
    #[serde(default)]
    pub jaeger: Jaeger,

    /// Language the service is written in
    pub language: Option<String>,

    // Kubernetes specific flags

    /// Chart to use for the service
    #[serde(default)]
    pub chart: String,

    /// Resource limits and requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Resources>,
    /// Replication limits
    #[serde(default)]
    pub replicaCount: Option<u32>,
    /// host aliases to inject in /etc/hosts
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hostAliases: Vec<HostAlias>,
    /// Environment variables to inject
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Config files to inline in a configMap
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configs: Option<ConfigMap>,
    /// Volumes mounts
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumeMounts: Vec<VolumeMount>,
    /// Init container intructions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub initContainers: Vec<InitContainer>,
    /// Http Port to expose
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub httpPort: Option<u32>,
    /// Vault options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault: Option<VaultOpts>,
    /// Health check parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<HealthCheck>,
    /// Service dependencies
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Dependency>,
    /// Regions service is deployed to
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<String>,
    /// Volumes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumes: Vec<Volume>,
    /// CronJobs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cronJobs: Vec<CronJob>,



    /// Service annotations (for internal services only)
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub serviceAnnotations: BTreeMap<String, String>,

    // TODO: boot time -> minReadySeconds


    /// Prometheus metric options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prometheus: Option<Prometheus>,

    /// Dashboards to generate
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dashboards: BTreeMap<String, Dashboard>,

    // TODO: logging alerts

    // TODO: stop hook
    //preStopHookPath: /die

    // Decoded secrets
    #[serde(default, skip_serializing, skip_deserializing)]
    pub _decoded_secrets: BTreeMap<String, String>,
}

impl Manifest {
    pub fn new(name: &str) -> Manifest {
        Manifest {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Walk the services directory and return the available services
    pub fn available() -> Result<Vec<String>> {
        let svcsdir = Path::new(".").join("services");
        let svcs = WalkDir::new(&svcsdir)
            .min_depth(1)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir());

        let mut xs = vec![];
        for e in svcs {
            let mut cmps = e.path().components();
            cmps.next(); // .
            cmps.next(); // services
            let svccomp = cmps.next().unwrap();
            let svcname = svccomp.as_os_str().to_str().unwrap();
            xs.push(svcname.into());
        }
        Ok(xs)
    }

    /// Read a manifest file in an arbitrary path
    fn read_from(pwd: &PathBuf) -> Result<Manifest> {
        let mpath = pwd.join("shipcat.yml");
        trace!("Using manifest in {}", mpath.display());
        if !mpath.exists() {
            bail!("Manifest file {} does not exist", mpath.display())
        }
        let mut f = File::open(&mpath)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        Ok(serde_yaml::from_str(&data)?)
    }


    /// Add implicit defaults to self
    fn implicits(&mut self, conf: &Config, region: Option<String>) -> Result<()> {
        if self.image.is_none() {
            // image name defaults to some prefixed version of the service name
            self.image = Some(format!("{}/{}", conf.defaults.imagePrefix, self.name))
        }

        if let Some(r) = region {
            if conf.regions.get(&r).is_none() {
                bail!("Unknown region {} in regions in config", r);
            }
            let reg = conf.regions[&r].clone(); // must exist
            // allow overriding tags
            if self.version.is_none() {
                trace!("overriding image.version with {:?}", reg.defaults.version);
                self.version = Some(reg.defaults.version);
            }
            for (k, v) in reg.env {
                self.env.insert(k, v);
            }
        }
        if self.chart == "" {
            self.chart = conf.defaults.chart.clone();
        }
        if self.replicaCount.is_none() {
            self.replicaCount = Some(conf.defaults.replicaCount)
        }

        // config map implicit name
        if let Some(ref mut cfg) = self.configs {
            if cfg.name.is_none() {
                cfg.name = Some(format!("{}-config", self.name));
            }
        }

        for d in &mut self.dependencies {
            if d.api.is_none() {
                d.api = Some("v1".to_string());
            }
        }

        Ok(())
    }

    /// Merge defaults from partial override file
    ///
    /// Note this does not merge all keys, because not everyting makes sense to
    /// override. E.g. service name.
    ///
    /// Currently being conservative and only allowing doing environment overrides for:
    /// - environment variables
    /// - image repo and default tag
    fn merge(&mut self, pth: &PathBuf) -> Result<()> {
        trace!("Merging {}", pth.display());
        if !pth.exists() {
            bail!("Defaults file {} does not exist", pth.display())
        }
        let mut f = File::open(&pth)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        // Because Manifest has most things implementing Default via serde
        // we can put this straight into a Manifest struct
        let mf: Manifest = serde_yaml::from_str(&data)?;

        // merge evars (most common override)
        for (k,v) in mf.env {
            self.env.entry(k).or_insert(v);
        }

        // maybe environment specific resources?
        // probably not a good idea
        //if self.resources.is_none() && mf.resources.is_some() {
        //    self.resources = mf.resources.clone();
        //}
        //if let Some(ref mut res) = self.resources {
        //    if res.limits.is_none() {
        //        res.limits = mf.resources.clone().unwrap().limits;
        //    }
        //    if res.requests.is_none() {
        //        res.requests = mf.resources.clone().unwrap().requests;
        //    }
        //    // for now: if limits or requests are specified, you have to fill in both CPU and memory
        //}

        // allow overriding of init containers
        if !mf.initContainers.is_empty() {
            self.initContainers = mf.initContainers.clone();
        }

        // allow overriding of host aliases
        if !mf.hostAliases.is_empty() {
            for hostAlias in &mf.hostAliases {
                if hostAlias.ip == "" || hostAlias.hostnames.is_empty() {
                    bail!("Host alias should have an ip and at least one hostname");
                }
            }
            trace!("overriding hostAliases with {:?}", mf.hostAliases);
            self.hostAliases = mf.hostAliases;
        }

        Ok(())
    }

    // Populate placeholder fields with secrets from vault
    fn secrets(&mut self, client: &Vault, region: &str) -> Result<()> {
        // some services use keys from other services
        let svc = if let Some(ref vopts) = self.vault {
            vopts.name.clone()
        } else {
            self.name.clone()
        };
        debug!("Injecting secrets from vault {}/{}", region, svc);

        // iterate over key value evars and replace placeholders
        for (k, v) in &mut self.env {
            if v == "IN_VAULT" {
                let vkey = format!("{}/{}/{}", region, svc, k);
                let secret = client.read(&vkey)?;
                *v = secret.clone();
                self._decoded_secrets.insert(vkey, secret);
            }
        }
        Ok(())
    }

    /// Fill in env overrides and populate secrets
    pub fn fill(&mut self, conf: &Config, region: &str, vault: &Option<Vault>) -> Result<()> {
        self.implicits(conf, Some(region.into()))?;
        if let &Some(ref client) = vault {
            self.secrets(&client, region)?;
        }

        // merge service specific env overrides if they exists
        let envlocals = Path::new(".")
            .join("services")
            .join(&self.name)
            .join(format!("{}.yml", region));
        if envlocals.is_file() {
            debug!("Merging environment locals from {}", envlocals.display());
            self.merge(&envlocals)?;
        }
        Ok(())
    }

    /// Complete (filled in env overrides and populate secrets) a manifest
    pub fn completed(region: &str, conf: &Config, service: &str, vault: Option<Vault>) -> Result<Manifest> {
        let pth = Path::new(".").join("services").join(service);
        if !pth.exists() {
            bail!("Service folder {} does not exist", pth.display())
        }
        let mut mf = Manifest::read_from(&pth)?;
        mf.fill(conf, &region, &vault)?;
        Ok(mf)
    }

    /// A super base manifest - from an unknown region
    pub fn basic(service: &str, conf: &Config, region: Option<String>) -> Result<Manifest> {
        let pth = Path::new(".").join("services").join(service);
        if !pth.exists() {
            bail!("Service folder {} does not exist", pth.display())
        }
        let mut mf = Manifest::read_from(&pth)?;
        if mf.name != service {
            bail!("Service name must equal the folder name");
        }
        mf.implicits(conf, region)?;
        Ok(mf)
    }

    /// Print manifest to debug output
    pub fn print(&self) -> Result<()> {
        let encoded = serde_yaml::to_string(self)?;
        trace!("{}\n", encoded);
        Ok(())
    }

    /// Verify assumptions about manifest
    ///
    /// Assumes the manifest has been populated with `implicits`
    pub fn verify(&self, conf: &Config) -> Result<()> {
        // limit to 40 characters, alphanumeric, dashes for sanity.
        let re = Regex::new(r"^[0-9a-z\-]{1,40}$").unwrap();
        if !re.is_match(&self.name) {
            bail!("Please use a short, lower case service names with dashes");
        }
        if self.name.ends_with('-') || self.name.starts_with('-') {
            bail!("Please use dashes to separate words only");
        }

        for d in &self.dataHandling {
            d.verify()?;
        }

        if self.external {
            warn!("Ignoring most validation for kube-external service {}", self.name);
            return Ok(());
        }

        // run the `Verify` trait on all imported structs
        // mandatory structs first
        if let Some(ref r) = self.resources {
            r.verify()?;
        } else {
            // TODO: maybe not for external services
            bail!("Resources is mandatory");
        }

        // optional/vectorised entries
        for d in &self.dependencies {
            d.verify()?;
        }
        for ha in &self.hostAliases {
            ha.verify()?;
        }
        for ic in &self.initContainers {
            ic.verify()?;
        }
        if let Some(ref cmap) = self.configs {
            cmap.verify()?;
        }

        // misc minor properties
        if self.replicaCount.unwrap() == 0 {
            bail!("Need replicaCount to be at least 1");
        }

        // TODO: verify self.image exists!

        // regions must have a defaults file in ./environments
        for r in &self.regions {
            if conf.regions.get(r).is_none() {
                bail!("Unsupported region {} without entry in config", r);
            }
        }
        if self.regions.is_empty() {
            bail!("No regions specified for {}", self.name);
        }


        // health check
        // every service that exposes http MUST have a health check
        if self.httpPort.is_some() && self.health.is_none() {
            bail!("{} has an httpPort but no health check", self.name)
        }

        // add some warnigs about missing health checks and ports regardless
        // TODO: make both mandatory once we have sidecars supported
        if self.httpPort.is_none() {
            warn!("{} exposes no http port", self.name);
        }
        if self.health.is_none() {
            warn!("{} does not set a health check", self.name)
        }

        if !self.serviceAnnotations.is_empty() {
            warn!("serviceAnnotation is an experimental/temporary feature")
        }

        Ok(())
    }
}


/// Validate the manifest of a service in the services directory
///
/// This will populate the manifest for all supported environments,
/// and `verify` their parameters.
/// Optionally, it will also verify that all secrets are found in the corresponding
/// vault locations serverside (which require vault credentials).
pub fn validate(services: Vec<String>, conf: &Config, region: String, vault: Option<Vault>) -> Result<()> {
    for svc in services {
        let mut mf = Manifest::basic(&svc, conf, Some(region.clone()))?;
        if mf.regions.contains(&region) {
            info!("validating {} for {}", svc, region);
            mf.fill(&conf, &region, &vault)?;
            mf.verify(&conf)?;
            info!("validated {} for {}", svc, region);
            mf.print()?; // print it if sufficient verbosity
        } else if mf.external {
             mf.verify(&conf)?; // exits early - but will verify some stuff
        } else {
            bail!("{} is not configured to be deployed in {}", svc, region)
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate};
    use tests::setup;
    use super::Vault;
    use super::Config;

    #[test]
    fn graph_generate() {
        setup();
        let client = Vault::default().unwrap();
        let conf = Config::read().unwrap();
        let res = validate(vec!["fake-ask".into()], &conf, "dev-uk".into(), Some(client));
        assert!(res.is_ok());
        let res2 = validate(vec!["fake-storage".into(), "fake-ask".into()], &conf, "dev-uk".into(), None);
        assert!(res2.is_ok())
    }
}
