#![allow(non_snake_case)]

use serde_yaml;
use walkdir::WalkDir;
use regex::Regex;

use std::io::prelude::*;
use std::fs::File;
use std::path::{PathBuf, Path};
use std::collections::BTreeMap;

use super::Result;
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

    /// Optional image name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Optional version/tag of docker image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Optional image command (if not using the default docker command)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

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

    /// Namepace
    #[serde(default = "namespace_default")]
    pub namespace: String,

    /// Chart to use for the service
    #[serde(default = "chart_default")]
    pub chart: String,

    /// Resource limits and requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Resources>,
    /// Replication limits
    #[serde(default = "replica_count_default")]
    pub replicaCount: u32,
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

    // Internal path of this manifest
    #[serde(skip_serializing, skip_deserializing)]
    _path: String,

    // Internal location this manifest is intended for
    #[serde(skip_serializing, skip_deserializing)]
    pub _location: String,

    // Decoded secrets
    #[serde(default, skip_serializing, skip_deserializing)]
    pub _decoded_secrets: BTreeMap<String, String>,
}
fn chart_default() -> String { "base".into() }
fn namespace_default() -> String { "dev".into() } // TODO: from a config file!
fn replica_count_default() -> u32 { 2 } // TODO: 1?



impl Manifest {
    pub fn new(name: &str, location: &PathBuf) -> Manifest {
        Manifest {
            name: name.into(),
            _path: location.to_string_lossy().into(),
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
        let mut res: Manifest = serde_yaml::from_str(&data)?;
        // store the location internally (not serialized to disk)
        res._path = mpath.to_string_lossy().into();
        Ok(res)
    }


    /// Add implicit defaults to self
    fn implicits(&mut self) -> Result<()> {
        // image name defaults to the service name
        if self.image.is_none() {
            self.image = Some(format!("quay.io/babylonhealth/{}", self.name))
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

        // allow overriding tags
        if let Some(tag) = mf.version {
            if self.version.is_none() {
                trace!("overriding image.version with {:?}", tag);
                self.version = Some(tag);
            };
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

        //if self.volumeMounts.is_empty() && !mf.volumeMounts.is_empty() {
        //    self.volumeMounts = mf.volumeMounts;
        //}
        //if self.initContainers.is_empty() && !mf.initContainers.is_empty() {
        //    self.initContainers = mf.initContainers.clone();
        //}

        //if self.volumes.is_empty() && !mf.volumes.is_empty() {
        //    self.volumes = mf.volumes;
        //}

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
    fn secrets(&mut self, client: &mut Vault, region: &str) -> Result<()> {
        // some services use keys from other services
        let svc = if let Some(ref vopts) = self.vault {
            vopts.name.clone()
        } else {
            self.name.clone()
        };
        debug!("Injecting secrets from vault {}/{}", region, svc);

        // iterate over key value evars and replace placeholders
        for (k, v) in &mut self.env {
            let kube_prefix = "IN_KUBE_SECRETS";

            if v == "IN_VAULT" {
                let vkey = format!("{}/{}/{}", region, svc, k);
                let secret = client.read(&vkey)?;
                *v = secret.clone();
                self._decoded_secrets.insert(vkey, secret);
            } else if v.starts_with(kube_prefix) {
                let res = if v == kube_prefix {
                    // no extra info -> assume same kube secret name as evar name
                    k.to_string()
                } else {
                    // key after :, split and return second half
                    assert!(v.contains(':'));
                    let parts : Vec<_> = v.split(':').collect();
                    if parts[1].is_empty() {
                        bail!("{} does not have a valid key path", v.clone());
                    }
                    parts[1].to_string()
                };
                *v = format!("kube-secret-{}", res.to_lowercase().replace("_", "-"));
            }
        }
        Ok(())
    }

    /// Fill in env overrides and populate secrets
    pub fn fill(&mut self, region: &str, vault: Option<&mut Vault>) -> Result<()> {
        self.implicits()?;
        if let Some(client) = vault {
            self.secrets(client, region)?;
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
        // merge global environment defaults if they exist
        let envglobals = Path::new(".")
            .join("environments")
            .join(format!("{}.yml", region));
        if envglobals.is_file() {
            debug!("Merging environment globals from {}", envglobals.display());
            self.merge(&envglobals)?;
        }
        Ok(())
    }

    /// Complete (filled in env overrides and populate secrets) a manifest
    pub fn completed(region: &str, service: &str, vault: Option<&mut Vault>) -> Result<Manifest> {
        let pth = Path::new(".").join("services").join(service);
        if !pth.exists() {
            bail!("Service folder {} does not exist", pth.display())
        }
        let mut mf = Manifest::read_from(&pth)?;
        mf.fill(&region, vault)?;
        Ok(mf)
    }

    /// A super base manifest - from an unknown region
    pub fn basic(service: &str) -> Result<Manifest> {
        let pth = Path::new(".").join("services").join(service);
        if !pth.exists() {
            bail!("Service folder {} does not exist", pth.display())
        }
        let mut mf = Manifest::read_from(&pth)?;
        if mf.name != service {
            bail!("Service name must equal the folder name");
        }
        mf.implicits()?;
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
    pub fn verify(&self) -> Result<()> {
        // limit to 40 characters, alphanumeric, dashes for sanity.
        let re = Regex::new(r"^[0-9a-z\-]{1,40}$").unwrap();
        if !re.is_match(&self.name) {
            bail!("Please use a short, lower case service names with dashes");
        }
        if self.name.ends_with('-') || self.name.starts_with('-') {
            bail!("Please use dashes to separate words only");
        }

        // run the `Verify` trait on all imported structs
        // mandatory structs first:
        self.resources.clone().unwrap().verify()?;

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
        for d in &self.dataHandling {
            d.verify()?;
        }
        if let Some(ref cmap) = self.configs {
            cmap.verify()?;
        }

        // misc minor properties
        if self.replicaCount == 0 {
            bail!("Need replicaCount to be at least 1");
        }

        // regions must have a defaults file in ./environments
        for r in &self.regions {
            let regionfile = Path::new(".")
                .join("environments")
                .join(format!("{}.yml", r));

            if ! regionfile.is_file() {
                bail!("Unsupported region {} without region file {}",
                    r, regionfile.display());
            }
            let region_parts : Vec<_> = r.split('-').collect();
            if region_parts.len() != 2 {
                bail!("invalid region {} of len {}", r, r.len());
            };
            // TODO: verify allowed namespaces per region
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
pub fn validate(services: Vec<String>, region: String, secrets: bool) -> Result<()> {
    // vault client (if needed)
    let mut v = Vault::default().unwrap(); // any set evars will succeed here
    v.mock_secrets(); // not needed for output

    for svc in services {
        let mut mf = Manifest::basic(&svc)?;
        if mf.regions.contains(&region) {
            info!("validating {} for {}", svc, region);
            mf.fill(&region, if secrets { Some(&mut v) } else { None })?;
            mf.verify()?;
            info!("validated {} for {}", svc, region);
            mf.print()?; // print it if sufficient verbosity
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

    #[test]
    fn graph_generate() {
        setup();
        let res = validate(vec!["fake-ask".into()], "dev-uk".into(), true);
        assert!(res.is_ok());
        let res2 = validate(vec!["fake-storage".into(), "fake-ask".into()], "dev-uk".into(), false);
        assert!(res2.is_ok())
    }
}
