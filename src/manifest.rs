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
use super::structs::{CronJob, Kong, Sidecar};
use super::structs::Worker;

/// Main manifest, serializable from shipcat.yml
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Name of the service
    #[serde(default)]
    pub name: String,

    // Region injected in helm chart
    #[serde(default, skip_deserializing)]
    pub region: String,

    // Environment (kube namespace) injected in helm chart
    #[serde(default, skip_deserializing)]
    pub environment: String,

    /// Wheter to ignore this service
    #[serde(default, skip_serializing)]
    pub disabled: bool,
    /// Wheter the service is externally managed
    #[serde(default, skip_serializing)]
    pub external: bool,

    /// Optional image name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    /// Optional uncompressed image size (for estimating helm timeouts)
    #[serde(skip_serializing, default = "default_image_size")]
    pub imageSize: u32,

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
    #[serde(default, skip_serializing)]
    pub dataHandling: DataHandling,

    /// Jaeger options
    #[serde(default)]
    pub jaeger: Jaeger,

    /// Language the service is written in
    #[serde(skip_serializing)]
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
    #[serde(default, skip_serializing)]
    pub regions: Vec<String>,
    /// Volumes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumes: Vec<Volume>,
    /// CronJobs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cronJobs: Vec<CronJob>,

    /// Sidecars
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sidecars: Vec<Sidecar>,

    /// Worker side-deployments
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workers: Vec<Worker>,

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

    /// Kong config
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kong: Option<Kong>,

    /// Kube Secret Files to append
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secretFiles: BTreeMap<String, String>,

    // TODO: logging alerts

    // TODO: stop hook
    //preStopHookPath: /die

    // Decoded secrets
    #[serde(default, skip_serializing, skip_deserializing)]
    pub _decoded_secrets: BTreeMap<String, String>,
/*
    // Internal type
    #[serde(default, skip_serializing, skip_deserializing)]
    pub _type: ManifestType,
*/
}

fn default_image_size() -> u32 { 512 }

/*
/// What an internal representation of a `Manifest` means
pub enum ManifestType {
    /// A plain shipcat.yml with just `Manifest::implicits` filled in
    Basic,
    /// A shipcat.yml with region, config overrides applied - potentially secrets
    Complete,
    /// Completed manifest with mocked secrets
    CompleteMocked,
    /// A completed manifest with templated configs inlined for helm
    HelmValues,
}
impl Default for ManifestType {
    fn default() -> Self {
        ManifestType::Basic
    }
}*/

impl Manifest {
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
    fn pre_merge_implicits(&mut self, conf: &Config) -> Result<()> {
        if self.image.is_none() {
            // image name defaults to some prefixed version of the service name
            self.image = Some(format!("{}/{}", conf.defaults.imagePrefix, self.name))
        }

        if self.chart == "" {
            self.chart = conf.defaults.chart.clone();
        }
        if self.replicaCount.is_none() {
            self.replicaCount = Some(conf.defaults.replicaCount)
        }

        // dataHandling has cascading encryption values
        self.dataHandling.implicits();

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

    /// Add implicit defaults to self after merging in region overrides
    fn post_merge_implicits(&mut self, conf: &Config, region: Option<String>) -> Result<()> {
        if let Some(r) = region {
            if conf.regions.get(&r).is_none() {
                bail!("Unknown region {} in regions in config", r);
            }
            self.region = r.clone();
            let reg = conf.regions[&r].clone(); // must exist
            for (k, v) in reg.env {
                self.env.insert(k, v);
            }

            // Kong has implicit, region-scoped values
            if let Some(ref mut kong) = self.kong {
                kong.implicits(self.name.clone(), conf.regions[&r].clone());
            }

            // Inject the region environment
            self.environment = reg.defaults.environment;
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

        // merge evars (overwrites evars found in shipcat.yml)
        for (k,v) in mf.env {
            self.env.insert(k, v);
        }
        // Must override Kong per environment (overwrite full struct)
        if mf.kong.is_some() {
            if self.kong.is_some() {
                bail!("Cannot have kong in main shipcat.yml and environment override files");
            }
            // Because this struct is overridden entirly; call implicits on this struct as well
            self.kong = mf.kong.clone();
        }
        // Version overrides (can be locked across envs, but overwrite when requested)
        if mf.version.is_some() {
            self.version = mf.version;
        }
        // Allow overriding resources (full struct only)
        if mf.resources.is_some(){
            self.resources = mf.resources;
        }
        // allow overriding of init containers (full vector only)
        if !mf.initContainers.is_empty() {
            self.initContainers = mf.initContainers.clone();
        }
        // allow overriding of host aliases (full vector only)
        if !mf.hostAliases.is_empty() {
            for hostAlias in &mf.hostAliases {
                if hostAlias.ip == "" || hostAlias.hostnames.is_empty() {
                    bail!("Host alias should have an ip and at least one hostname");
                }
            }
            trace!("overriding hostAliases with {:?}", mf.hostAliases);
            self.hostAliases = mf.hostAliases;
        }
        // TODO: more as becomes needed

        Ok(())
    }

    fn get_vault_path(&self, region: &str) -> String {
        // some services use keys from other services
        let (svc, reg) = if let Some(ref vopts) = self.vault {
            (vopts.name.clone(), vopts.region.clone().unwrap_or_else(|| region.into()))
        } else {
            (self.name.clone(), region.into())
        };
        format!("{}/{}", reg, svc)
    }

    // Populate placeholder fields with secrets from vault
    fn secrets(&mut self, client: &Vault, region: &str) -> Result<()> {
        let pth = self.get_vault_path(region);
        debug!("Injecting secrets from vault {}", pth);

        // iterate over key value evars and replace placeholders
        for (k, v) in &mut self.env {
            if v == "IN_VAULT" {
                let vkey = format!("{}/{}", pth, k);
                let secret = client.read(&vkey)?;
                *v = secret.clone();
                self._decoded_secrets.insert(vkey, secret);
            }
        }
        // do the same for secret secrets
        for (k, v) in &mut self.secretFiles {
            if v == "IN_VAULT" {
                let vkey = format!("{}/{}", pth, k);
                let secret = client.read(&vkey)?;
                *v = secret.clone();
                self._decoded_secrets.insert(vkey, secret);
            }
        }
        Ok(())
    }


    fn verify_secrets_exist(&self, region: &str) -> Result<()> {
        // what are we requesting
        let keys = self.env.clone().into_iter()
            .filter(|(_,v)| v == "IN_VAULT")
            .map(|(k,_)| k)
            .collect::<Vec<_>>();
        let files = self.secretFiles.clone().into_iter()
            .filter(|(_,v)| v == "IN_VAULT")
            .map(|(k, _)| k)
            .collect::<Vec<_>>();
        if keys.is_empty() && files.is_empty() {
            return Ok(()); // no point trying to cross reference
        }

        // what we have
        let v = Vault::masked()?; // masked doesn't matter - only listing anyway
        let secpth = self.get_vault_path(region);
        let found = v.list(&secpth)?; // can fail if folder is empty
        debug!("Found secrets {:?} for {}", found, self.name);

        // compare
        for k in keys {
            if !found.contains(&k) {
                bail!("Secret {} not found in vault {} for {}", k, secpth, self.name);
            }
        }
        for k in files {
            if !found.contains(&k) {
                bail!("Secret file {} not found in vault {} for {}", k, secpth, self.name);
            }
        }
        Ok(())
    }

    /// Fill in env overrides and populate secrets
    fn fill(&mut self, conf: &Config, region: &str, vault: &Option<Vault>) -> Result<()> {
        self.pre_merge_implicits(conf)?;
        // merge service specific env overrides if they exists
        let envlocals = Path::new(".")
            .join("services")
            .join(&self.name)
            .join(format!("{}.yml", region));
        if envlocals.is_file() {
            debug!("Merging environment locals from {}", envlocals.display());
            self.merge(&envlocals)?;
        }
        self.post_merge_implicits(conf, Some(region.into()))?;
        if let &Some(ref client) = vault {
            self.secrets(&client, region)?;
        }
        Ok(())
    }

    /// Complete (filled in env overrides and populate secrets) a manifest
    pub fn completed(service: &str, conf: &Config, region: &str) -> Result<Manifest> {
        let v = Vault::default()?;
        let pth = Path::new(".").join("services").join(service);
        if !pth.exists() {
            bail!("Service folder {} does not exist", pth.display())
        }
        let mut mf = Manifest::read_from(&pth)?;
        mf.fill(conf, &region, &Some(v))?;
        mf.inline_configs()?;
        Ok(mf)
    }

    /// Mostly completed but stubbed secrets version of the manifest
    pub fn stubbed(service: &str, conf: &Config, region: &str) -> Result<Manifest> {
        let pth = Path::new(".").join("services").join(service);
        if !pth.exists() {
            bail!("Service folder {} does not exist", pth.display())
        }
        let mut mf = Manifest::read_from(&pth)?;
        mf.fill(conf, &region, &None)?;
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
        mf.pre_merge_implicits(conf)?;
        // not merging here, but do all implicts we can anyway
        mf.post_merge_implicits(conf, region)?;
        Ok(mf)
    }

    /// Inline templates in values
    fn inline_configs(&mut self) -> Result<()> {
        use super::template;
        use tera::Context;
        let rdr = template::service_bound_renderer(&self.name)?;
        if let Some(ref mut cfg) = self.configs {
            for f in &mut cfg.files {
                let mut ctx = Context::new();
                ctx.add("env", &self.env.clone());
                ctx.add("service", &self.name.clone());
                ctx.add("region", &self.region.clone());
                f.value = Some((rdr)(&f.name, &ctx)?);
            }
        }
        Ok(())
    }

    pub fn estimate_wait_time(&self) -> u32 {
        // 512 default => extra 60s wait
        let pulltimeestimate = (((self.imageSize*60) as f64)/(1024 as f64)) as u32;
        let rcount = self.replicaCount.unwrap(); // this is set by defaults!
        // NB: we wait to pull on each node because of how rolling-upd
        if let Some(ref hc) = self.health {
            // wait for at most (bootTime + pulltimeestimate) * replicas
            (hc.wait + pulltimeestimate) * rcount
        } else {
            // sensible guess for boot time (helm default is 300 without any context)
            (30 + pulltimeestimate) * rcount
        }
    }



    /// Override version with an optional one from the CLI
    pub fn set_version(mut self, ver: &Option<String>) -> Self {
        if ver.is_some() {
            self.version = ver.clone(); // override version here if set
        }
        self
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
        assert!(self.region != ""); // needs to have been set by implicits!
        // limit to 40 characters, alphanumeric, dashes for sanity.
        let re = Regex::new(r"^[0-9a-z\-]{1,40}$").unwrap();
        if !re.is_match(&self.name) {
            bail!("Please use a short, lower case service names with dashes");
        }
        if self.name.ends_with('-') || self.name.starts_with('-') {
            bail!("Please use dashes to separate words only");
        }

        self.dataHandling.verify(&conf)?;
        if let Some(ref md) = self.metadata {
            md.verify(&conf)?;
        }

        if self.external {
            warn!("Ignoring most validation for kube-external service {}", self.name);
            return Ok(());
        }

        // run the `Verify` trait on all imported structs
        // mandatory structs first
        if let Some(ref r) = self.resources {
            r.verify(&conf)?;
        } else {
            // TODO: maybe not for external services
            bail!("Resources is mandatory");
        }

        // optional/vectorised entries
        for d in &self.dependencies {
            d.verify(&conf)?;
        }
        for ha in &self.hostAliases {
            ha.verify(&conf)?;
        }
        for ic in &self.initContainers {
            ic.verify(&conf)?;
        }
        for wrk in &self.workers {
            wrk.verify(&conf)?;
        }
        if let Some(ref cmap) = self.configs {
            cmap.verify(&conf)?;
        }

        // misc minor properties
        if self.replicaCount.unwrap() == 0 {
            bail!("Need replicaCount to be at least 1");
        }

        // Env values are uppercase
        for (k, _) in &self.env {
            if k != &k.to_uppercase()  {
                bail!("Env vars need to be uppercase, found: {}", k);
            }
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
        if self.imageSize == 0{
            bail!("imageSize must be set in MB (default 512)");
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

// Validate the secrets exists in all regions
pub fn validate_secret_presence(conf: &Config, regions: Vec<String>) -> Result<()> {
    for r in regions {
        info!("validating secrets in {}", r);
        let services = Manifest::available()?;
        for svc in services {
            let mut mf = Manifest::basic(&svc, conf, Some(r.clone()))?;
            if mf.regions.contains(&r) && !mf.external && !mf.disabled {
                info!("validating secrets for {} in {}", svc, r);
                mf.fill(&conf, &r, &None)?;
                mf.verify_secrets_exist(&r)?;
            }
        }
    }
    Ok(())
}

/// Show GDPR related info for a service
///
/// Prints the cascaded structs from a manifests `dataHandling`
pub fn gdpr_show(svc: &str, conf: &Config, region: &str) -> Result<()> {
    use std::io::{self, Write};
    let mf = Manifest::stubbed(svc, conf, region)?;
    let out = serde_yaml::to_string(&mf.dataHandling)?;
    let _ = io::stdout().write(format!("{}\n", out).as_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate};
    use tests::setup;
    use super::Vault;
    use super::Config;
    use super::Manifest;
    use super::HealthCheck;

    #[test]
    fn validate_test() {
        setup();
        let client = Vault::default().unwrap();
        let conf = Config::read().unwrap();
        let res = validate(vec!["fake-ask".into()], &conf, "dev-uk".into(), Some(client));
        assert!(res.is_ok());
        let res2 = validate(vec!["fake-storage".into(), "fake-ask".into()], &conf, "dev-uk".into(), None);
        assert!(res2.is_ok())
    }

    #[test]
    fn wait_time_check() {
        setup();
        // DEFAULT SETUP: no values == defaults => 120s helm wait
        let mut mf = Manifest::default();
        mf.imageSize = 512;
        mf.health = Some(HealthCheck {
            uri: "/".into(),
            wait: 30,
            ..Default::default()
        });
        mf.replicaCount = Some(2);
        let wait = mf.estimate_wait_time();
        assert_eq!(wait, 30*2*2);

        // setup with large image and short boot time:
        mf.imageSize = 4096;
        mf.health = Some(HealthCheck {
            uri: "/".into(),
            wait: 20,
            ..Default::default()
        });
        let wait2 = mf.estimate_wait_time();
        assert_eq!(wait2, (20+240)*2);
    }

    #[test]
    fn manifest_test() {
        setup();
        let conf = Config::read().unwrap();
        let mf = Manifest::basic("fake-storage", &conf, Some("dev-uk".into())).unwrap();
        // verify datahandling implicits
        let s3 = mf.dataHandling.stores[0].clone();
        assert!(s3.encrypted.unwrap());
        assert_eq!(s3.fields[0].encrypted.unwrap(), false); // overridden
        assert_eq!(s3.fields[1].encrypted.unwrap(), true); // cascaded
        assert_eq!(s3.fields[0].keyRotator, None); // not set either place
        assert_eq!(s3.fields[1].keyRotator, Some("2w".into())); // field value
    }
}
