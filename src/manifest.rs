#![allow(non_snake_case)]

use serde_yaml;
use regex::Regex;

use std::io::prelude::*;
use std::fs::File;
use std::path::{PathBuf, Path};
use std::collections::BTreeMap;

use super::Result;
use super::vault::Vault;

// All structs come from the structs directory
use super::structs::kube::*;
use super::structs::{Metadata, DataHandling, VaultOpts, Jaeger, Dependency, Image};
use super::structs::prometheus::{Prometheus, Dashboard};


/// Main manifest, serializable from shipcat.yml
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Manifest {
    /// Name of the service
    #[serde(default)]
    pub name: String,

    /// Wheter to ignore this service
    #[serde(default, skip_serializing)]
    pub disabled: bool,

    /// Optional image name (if different from service name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
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

    /// Namepace - dev or internal only
    #[serde(default = "namespace_default")]
    pub namespace: String,

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
}
fn namespace_default() -> String { "dev".into() }
fn replica_count_default() -> u32 { 2 } // TODO: 1?



impl Manifest {
    pub fn new(name: &str, location: &PathBuf) -> Manifest {
        Manifest {
            name: name.into(),
            _path: location.to_string_lossy().into(),
            ..Default::default()
        }
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
            self.image = Some(Image {
                name: Some(self.name.clone()),
                repository: None,
                tag: None,
            });
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

        // allow overriding image repository and tags
        if let Some(img) = mf.image {
            let mut curr = self.image.clone().unwrap();
            if curr.repository.is_none() {
                trace!("overriding image.repository with {:?}", img.repository);
                curr.repository = img.repository;
            }
            if curr.tag.is_none() {
                trace!("overriding image.tag with {:?}", img.tag);
                curr.tag = img.tag;
            }
            self.image = Some(curr);
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
                *v = secret;
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
        // set namespace property
        let region_parts : Vec<_> = region.split('-').collect();
        if region_parts.len() != 2 {
            bail!("invalid region {} of len {}", region, region.len());
        };
        self._location = region_parts[1].into();
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
        mf.implicits()?;
        Ok(mf)
    }

    /// Print manifest to debug output
    pub fn print(&self) -> Result<()> {
        let encoded = serde_yaml::to_string(self)?;
        debug!("{}\n", encoded);
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
        // TODO: verify namespace in allowed namespaces

        // 1. Verify resources
        // (We can unwrap all the values as we assume implicit called!)
        let req = self.resources.clone().unwrap().requests.unwrap().clone();
        let lim = self.resources.clone().unwrap().limits.unwrap().clone();
        let req_memory = parse_memory(&req.memory)?;
        let lim_memory = parse_memory(&lim.memory)?;
        let req_cpu = parse_cpu(&req.cpu)?;
        let lim_cpu = parse_cpu(&lim.cpu)?;

        // 1.1 limits >= requests
        if req_cpu > lim_cpu {
            bail!("Requested more CPU than what was limited");
        }
        if req_memory > lim_memory {
            bail!("Requested more memory than what was limited");
        }
        // 1.2 sanity numbers
        if req_cpu > 10.0 {
            bail!("Requested more than 10 cores");
        }
        if req_memory > 10.0*1024.0*1024.0*1024.0 {
            bail!("Requested more than 10 GB of memory");
        }
        if lim_cpu > 20.0 {
            bail!("CPU limit set to more than 20 cores");
        }
        if lim_memory > 20.0*1024.0*1024.0*1024.0 {
            bail!("Memory limit set to more than 20 GB of memory");
        }

        // 2. Replicas
        if self.replicaCount == 0 {
            bail!("Need replicaCount to be at least 1");
        }

        // 3. host aliases - only verify syntax
        for hostAlias in &self.hostAliases {
            // Commonly accepted hostname regex from https://stackoverflow.com/questions/106179/regular-expression-to-match-dns-hostname-or-ip-address
            let ip_re = Regex::new(r"^(([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])\.){3}([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])$").unwrap();
            if hostAlias.ip == "" || !ip_re.is_match(&hostAlias.ip){
                bail!("The ip address for the host alias is incorrect");
            }
            if hostAlias.hostnames.is_empty() {
                bail!("At least one hostname must be specified for the host alias");
            }
            for hostname in &hostAlias.hostnames {
                // Commonly accepted ip address regex from https://stackoverflow.com/questions/106179/regular-expression-to-match-dns-hostname-or-ip-address
                let host_re = Regex::new(r"^(([a-zA-Z0-9]|[a-zA-Z0-9][a-zA-Z0-9\-]*[a-zA-Z0-9])\.)*([A-Za-z0-9]|[A-Za-z0-9][A-Za-z0-9\-]*[A-Za-z0-9])$").unwrap();
                if !host_re.is_match(&hostname) {
                    bail!("The hostname {} is incorrect for {}", hostname, hostAlias.ip);
                }
            }
        }

        // 4. configs
        // 4.1) mount paths can't be empty string
        if let Some(ref cfgmap) = self.configs {
            if cfgmap.mount == "" || cfgmap.mount == "~" {
                bail!("Empty mountpath for {} mount ", cfgmap.name.clone().unwrap())
            }
            if !cfgmap.mount.ends_with('/') {
                bail!("Mount path '{}' for {} must end with a slash", cfgmap.mount, cfgmap.name.clone().unwrap());
            }
            for f in &cfgmap.files {
                if !f.name.ends_with(".j2") {
                    bail!("Only supporting templated config files atm")
                }
                // TODO: verify file exists? done later anyway
            }
        } else {
            warn!("No configs key in manifest");
            warn!("Did you use the old volumes key?");
        }

        // 5. volumes
        // TODO:

        // 6. dependencies
        for d in &self.dependencies {
            if d.name == "core-ruby" || d.name == "php-backend-monolith" {
                debug!("Depending on legacy {} monolith", d.name);
                continue;
            }
            // 5.a) d.name must exist in services/
            let dpth = Path::new(".").join("services").join(d.name.clone());
            if !dpth.is_dir() {
                bail!("Service {} does not exist in services/", d.name);
            }
            // 5.b) d.api must parse as an integer
            assert!(d.api.is_some(), "api version set by implicits");
            if let Some(ref apiv) = d.api {
                let vstr = apiv.chars().skip_while(|ch| *ch == 'v').collect::<String>();
                let ver : usize = vstr.parse()?;
                trace!("Parsed api version of dependency {} as {}", d.name.clone(), ver);
            }
            if d.protocol != "http" && d.protocol != "grpc" {
                bail!("Illegal dependency protocol {}", d.protocol)
            }
        }

        // 7. regions must have a defaults file in ./environments
        for r in &self.regions {
            let regionfile = Path::new(".")
                .join("environments")
                .join(format!("{}.yml", r));

            if ! regionfile.is_file() {
                bail!("Unsupported region {} without region file {}",
                    r, regionfile.display());
            }
        }
        if self.regions.is_empty() {
            bail!("No regions specified for {}", self.name);
        }

        // 8. init containers - only verify syntax
        for initContainer in &self.initContainers {
            let re = Regex::new(r"(?:[a-z]+/)?([a-z]+)(?::[0-9]+)?").unwrap();
            if !re.is_match(&initContainer.image) {
                bail!("The init container {} does not seem to match a valid image registry", initContainer.name);
            }
            if initContainer.command.is_empty() {
                bail!("A command must be specified for the init container {}", initContainer.name);
            }
        }

        // 9. health check
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

        // 10. data handling sanity
        // can't block on this yet
        for d in &self.dataHandling {
            if d.pii && !d.encrypted {
                warn!("{} stores PII without encryption", self.name)
            }
            if d.spii && !d.encrypted {
                warn!("{} stores SPII without encryption", self.name)
            }
        }


        Ok(())
    }
}

// Parse normal k8s memory resource value into floats
fn parse_memory(s: &str) -> Result<f64> {
    let digits = s.chars().take_while(|ch| ch.is_digit(10) || *ch == '.').collect::<String>();
    let unit = s.chars().skip_while(|ch| ch.is_digit(10) || *ch == '.').collect::<String>();
    let mut res : f64 = digits.parse()?;
    trace!("Parsed {} ({})", digits, unit);
    if unit == "Ki" {
        res *= 1024.0;
    } else if unit == "Mi" {
        res *= 1024.0*1024.0;
    } else if unit == "Gi" {
        res *= 1024.0*1024.0*1024.0;
    } else if unit == "k" {
        res *= 1000.0;
    } else if unit == "M" {
        res *= 1000.0*1000.0;
    } else if unit == "G" {
        res *= 1000.0*1000.0*1000.0;
    } else if unit != "" {
        bail!("Unknown unit {}", unit);
    }
    trace!("Returned {} bytes", res);
    Ok(res)
}

// Parse normal k8s cpu resource values into floats
// We don't allow power of two variants here
fn parse_cpu(s: &str) -> Result<f64> {
    let digits = s.chars().take_while(|ch| ch.is_digit(10) || *ch == '.').collect::<String>();
    let unit = s.chars().skip_while(|ch| ch.is_digit(10) || *ch == '.').collect::<String>();
    let mut res : f64 = digits.parse()?;

    trace!("Parsed {} ({})", digits, unit);
    if unit == "m" {
        res /= 1000.0;
    } else if unit == "k" {
        res *= 1000.0;
    } else if unit != "" {
        bail!("Unknown unit {}", unit);
    }
    trace!("Returned {} cores", res);
    Ok(res)
}

/// Validate the manifest of a service in the services directory
///
/// This will populate the manifest for all supported environments,
/// and `verify` their parameters.
/// Optionally, it will also verify that all secrets are found in the corresponding
/// vault locations serverside (which require vault credentials).
pub fn validate(service: &str, secrets: bool) -> Result<()> {
    let pth = Path::new(".").join("services").join(service);
    if !pth.exists() {
        bail!("Service folder {} does not exist", pth.display())
    }
    let mf = Manifest::read_from(&pth)?;
    if mf.name != service {
        bail!("Service name must equal the folder name");
    }
    for region in mf.regions.clone() {
        let mut mfr = mf.clone();
        if secrets {
            // need a new one for each region!
            let mut vault = Vault::default().unwrap();
            vault.mock_secrets(); // not needed for output
            mfr.fill(&region, Some(&mut vault))?;
        } else {
            mfr.fill(&region, None)?;
        }
        mfr.verify()?;
        info!("validated {} for {}", service, region);
        mfr.print()?; // print it if sufficient verbosity
    }
    Ok(())
}
