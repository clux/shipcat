use serde_yaml;
use walkdir::WalkDir;
use regex::Regex;
use base64;

use std::io::prelude::*;
use std::fs::File;
use std::path::{PathBuf, Path};
use std::collections::BTreeMap;

use super::{Result, Config, VaultConfig};
use super::vault::Vault;

// All structs come from the structs directory
use super::structs::traits::Verify;
use super::structs::{HealthCheck, ConfigMap};
use super::structs::{InitContainer, Resources, HostAlias};
use super::structs::volume::{Volume, VolumeMount};
use super::structs::{Metadata, VaultOpts, Dependency};
//use super::structs::prometheus::{Prometheus, Dashboard};
use super::structs::security::DataHandling;
use super::structs::Probe;
use super::structs::{CronJob, Sidecar};
use super::structs::{Kafka, Kong};
use super::structs::RollingUpdate;
use super::structs::autoscaling::AutoScaling;
use super::structs::tolerations::Tolerations;
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
    // Environment (not kube namespace) injected in helm chart
    #[serde(default, skip_deserializing)]
    pub environment: String,
    // Namespace (kube) injected in helm chart
    #[serde(default, skip_deserializing)]
    pub namespace: String,

    /// Wheter to ignore this service
    #[serde(default, skip_serializing)]
    pub disabled: bool,
    /// Wheter the service is externally managed
    #[serde(default, skip_serializing)]
    pub external: bool,
    /// Regions service is deployed to
    #[serde(default, skip_serializing)]
    pub regions: Vec<String>,
    /// Wheter the service should be public
    #[serde(default, skip_serializing)]
    pub publiclyAccessible: bool,

    // Secret evars
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secrets: BTreeMap<String, String>,

    // Decoded secrets - only used interally
    #[serde(default, skip_serializing, skip_deserializing)]
    pub _decoded_secrets: BTreeMap<String, String>,

    // BEFORE ADDING PROPERTIES READ THIS
    // Below are properties that can be merged, above ones that are global
    // if you add anything below here, also add it to merge.rs!

    /// Chart to use for the service
    #[serde(default)]
    pub chart: Option<String>,
    /// Optional image name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Optional uncompressed image size (for estimating helm timeouts)
    #[serde(skip_serializing)]
    pub imageSize: Option<u32>,
    /// Optional version/tag of docker image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Optional image command (if not using the default docker command)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,

    // misc metadata
    /// Canonical data sources like repo, docs, team names
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    /// Data sources and handling strategies
    #[serde(default, skip_serializing)]
    pub dataHandling: Option<DataHandling>,
    /// Language the service is written in
    #[serde(skip_serializing)]
    pub language: Option<String>,


    // Jaeger options CURRENTLY UNUSED
    //#[serde(default)]
    //pub jaeger: Jaeger,


    // Kubernetes specific flags

    /// Resource limits and requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Resources>,
    /// Replication limits
    #[serde(default)]
    pub replicaCount: Option<u32>,

    // our main abstractions for kube resources

    /// Environment variables to inject
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Config files to inline in a configMap
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configs: Option<ConfigMap>,
    /// Vault options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault: Option<VaultOpts>,
    /// Http Port to expose
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub httpPort: Option<u32>,
    /// Health check parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<HealthCheck>,
    /// Service dependencies
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Dependency>,

    /// Sidecars
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sidecars: Vec<Sidecar>,
    /// Worker side-deployments
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workers: Vec<Worker>,

    // pure kube yaml
    /// Optional readiness probe (REPLACES health abstraction)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readinessProbe: Option<Probe>,
    /// Optional liveness probe
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub livenessProbe: Option<Probe>,
    /// Rolling update Deployment parameters
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollingUpdate: Option<RollingUpdate>,
    /// Horizontal Pod Auto Scaler parameters
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoScaling: Option<AutoScaling>,
    /// Toleration parameters
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tolerations: Vec<Tolerations>,

    /// host aliases to inject in /etc/hosts
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hostAliases: Vec<HostAlias>,
    /// Volumes mounts
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumeMounts: Vec<VolumeMount>,
    /// Init container intructions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub initContainers: Vec<InitContainer>,

    /// Volumes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumes: Vec<Volume>,
    /// CronJobs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cronJobs: Vec<CronJob>,


    /// Service annotations (for internal services only)
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub serviceAnnotations: BTreeMap<String, String>,

    /// Extra labels
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,

    // Prometheus metric options CURRENTLY UNUSED
    //#[serde(skip_serializing_if = "Option::is_none")]
    //pub prometheus: Option<Prometheus>,

    // Dashboards to generate CURRENTLY UNUSED
    //#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    //pub dashboards: BTreeMap<String, Dashboard>,

    /// Kong config
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kong: Option<Kong>,

    /// Kafka config
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kafka: Option<Kafka>,

    /// Kube Secret Files to append
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secretFiles: BTreeMap<String, String>,

    // TODO: logging alerts
}

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

    fn get_vault_path(&self, vc: &VaultConfig) -> String {
        // some services use keys from other services
        let (svc, reg) = if let Some(ref vopts) = self.vault {
            (vopts.name.clone(), vopts.region.clone().unwrap_or_else(|| vc.folder.clone()))
        } else {
            (self.name.clone(), vc.folder.clone())
        };
        format!("{}/{}", reg, svc)
    }

    // Populate placeholder fields with secrets from vault
    fn secrets(&mut self, client: &Vault, vc: &VaultConfig) -> Result<()> {
        let pth = self.get_vault_path(vc);
        debug!("Injecting secrets from vault {}", pth);

        let special = "SHIPCAT_SECRET::".to_string();
        // iterate over key value evars and replace placeholders
        for (k, v) in &mut self.env {
            if v == "IN_VAULT" {
                let vkey = format!("{}/{}", pth, k);
                let secret = client.read(&vkey)?;
                self.secrets.insert(k.to_string(), secret.clone());
                self._decoded_secrets.insert(vkey, secret);
            }
            // Special cases that were handled by `| as_secret` template fn
            if v.starts_with(&special) {
                self.secrets.insert(k.to_string(), v.split_off(special.len()));
            }
        }
        // remove placeholders from env
        self.env = self.env.clone().into_iter()
            .filter(|&(_, ref v)| v != "IN_VAULT")
            .filter(|&(_, ref v)| !v.starts_with(&special))
            .collect();
        // do the same for secret secrets
        for (k, v) in &mut self.secretFiles {
            if v == "IN_VAULT" {
                let vkey = format!("{}/{}", pth, k);
                let secret = client.read(&vkey)?;
                *v = secret.clone();
                self._decoded_secrets.insert(vkey.clone(), secret);
                // sanity check; secretFiles are assumed base64 verify we can decode
                if base64::decode(v).is_err() {
                    bail!("Secret {} in vault is not base64 encoded", vkey);
                }
            }
        }
        Ok(())
    }

    pub fn verify_secrets_exist(&self, vc: &VaultConfig) -> Result<()> {
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
        let v = Vault::masked(vc)?; // masked doesn't matter - only listing anyway
        let secpth = self.get_vault_path(vc);
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

    /// Fill in env overrides and apply merge rules
    pub fn fill(&mut self, conf: &Config, region: &str) -> Result<()> {
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
        Ok(())
    }

    /// Complete (filled in env overrides and populate secrets) a manifest
    pub fn completed(service: &str, conf: &Config, region: &str) -> Result<Manifest> {
        let r = &conf.regions[region]; // tested for existence earlier
        let v = Vault::regional(&r.vault)?;
        let pth = Path::new(".").join("services").join(service);
        if !pth.exists() {
            bail!("Service folder {} does not exist", pth.display())
        }
        let mut mf = Manifest::read_from(&pth)?;
        // fill defaults and merge regions before extracting secrets
        mf.fill(conf, region)?;
        // replace one-off templates in evar strings with values
        mf.template_evars(conf, region)?;
        // secrets before configs (.j2 template files use raw secret values)
        mf.secrets(&v, &r.vault)?;
        // templates last
        mf.inline_configs(&conf, region)?;
        Ok(mf)
    }

    /// Mostly completed but stubbed secrets version of the manifest
    pub fn stubbed(service: &str, conf: &Config, region: &str) -> Result<Manifest> {
        let pth = Path::new(".").join("services").join(service);
        if !pth.exists() {
            bail!("Service folder {} does not exist", pth.display())
        }
        let mut mf = Manifest::read_from(&pth)?;
        mf.fill(conf, &region)?;
        Ok(mf)
    }

    /// Completed manifest with mocked values
    pub fn mocked(service: &str, conf: &Config, region: &str) -> Result<Manifest> {
        let r = &conf.regions[region]; // tested for existence earlier
        let v = Vault::mocked(&r.vault)?;
        let pth = Path::new(".").join("services").join(service);
        if !pth.exists() {
            bail!("Service folder {} does not exist", pth.display())
        }
        let mut mf = Manifest::read_from(&pth)?;
        // fill defaults and merge regions before extracting secrets
        mf.fill(conf, region)?;
        // replace one-off templates in evar strings with values
        mf.template_evars(conf, region)?;
        // (MOCKED) secrets before configs (.j2 template files use raw secret values)
        mf.secrets(&v, &r.vault)?;
        // templates last
        mf.inline_configs(&conf, region)?;
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

    // How long to wait for a kube rolling upgrade
    // Currently used by helm upgrade --wait
    pub fn estimate_wait_time(&self) -> u32 {
        // 512 default => extra 60s wait
        let pulltimeestimate = (((self.imageSize.unwrap()*60) as f64)/(1024 as f64)) as u32;
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
        let region = &conf.regions[&self.region]; // tested for existence earlier
        // limit to 50 characters, alphanumeric, dashes for sanity.
        // 63 is kube dns limit (13 char suffix buffer)
        let re = Regex::new(r"^[0-9a-z\-]{1,50}$").unwrap();
        if !re.is_match(&self.name) {
            bail!("Please use a short, lower case service names with dashes");
        }
        if self.name.ends_with('-') || self.name.starts_with('-') {
            bail!("Please use dashes to separate words only");
        }

        if let Some(ref dh) = self.dataHandling {
            dh.verify(&conf)?
        } // TODO: mandatory for later environments!

        if let Some(ref md) = self.metadata {
            md.verify(&conf)?;
        } else {
            bail!("Missing metadata for {}", self.name);
        }

        if self.external {
            warn!("Ignoring most validation for kube-external service {}", self.name);
            return Ok(());
        }

        if let Some(v) = &self.version {
            region.versioningScheme.verify(v)?;
        }

        // run the `Verify` trait on all imported structs
        // mandatory structs first
        if let Some(ref r) = self.resources {
            r.verify(&conf)?;
        } else {
            bail!("Resources is mandatory");
        }

        // optional/vectorised entries
        for d in &self.dependencies {
            d.verify(&conf)?;
        }
        for ha in &self.hostAliases {
            ha.verify(&conf)?;
        }
        for tl in &self.tolerations {
            tl.verify()?;
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
        if let Some(ref ru) = &self.rollingUpdate {
            ru.verify(self.replicaCount.unwrap())?;
        }

        // Env values are uppercase
        for (k, _) in &self.env {
            if k != &k.to_uppercase()  {
                bail!("Env vars need to be uppercase, found: {}", k);
            }
        }

        // internal errors - implicits set these!
        if self.image.is_none() {
            bail!("Image should be set at this point")
        }
        if self.imageSize.is_none() {
            bail!("imageSize must be set at this point");
        }
        if self.chart.is_none() {
            bail!("chart must be set at this point");
        }
        if self.namespace == "" {
            bail!("namespace must be set at this point");
        }

        // regions must have a defaults file in ./environments
        for r in &self.regions {
            if conf.regions.get(r).is_none() {
                bail!("Unsupported region {} without entry in config", r);
            }
        }
        if !self.regions.contains(&self.region.to_string()) {
            bail!("Unsupported region {} for service {}", self.region, self.name);
        }
        if self.regions.is_empty() {
            bail!("No regions specified for {}", self.name);
        }
        if self.environment == "" {
            bail!("Service {} ended up with an empty environment", self.name);
        }
        if self.namespace == "" {
            bail!("Service {} ended up with an empty namespace", self.name);
        }

        // health check
        // every service that exposes http MUST have a health check
        if self.httpPort.is_some() && (self.health.is_none() && self.readinessProbe.is_none()) {
            bail!("{} has an httpPort but no health check", self.name)
        }

        // add some warnigs about missing health checks and ports regardless
        // TODO: make both mandatory once we have sidecars supported
        if self.httpPort.is_none() {
            warn!("{} exposes no http port", self.name);
        }
        if self.health.is_none() && self.readinessProbe.is_none() {
            warn!("{} does not set a health check", self.name)
        }

        if !self.serviceAnnotations.is_empty() {
            warn!("serviceAnnotation is an experimental/temporary feature")
        }

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use tests::setup;
    use super::Config;
    use super::Manifest;
    use super::HealthCheck;

    #[test]
    fn wait_time_check() {
        setup();
        // DEFAULT SETUP: no values == defaults => 120s helm wait
        let mut mf = Manifest::default();
        mf.imageSize = Some(512);
        mf.health = Some(HealthCheck {
            uri: "/".into(),
            wait: 30,
            ..Default::default()
        });
        mf.replicaCount = Some(2);
        let wait = mf.estimate_wait_time();
        assert_eq!(wait, 30*2*2);

        // setup with large image and short boot time:
        mf.imageSize = Some(4096);
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
        let mf = Manifest::completed("fake-storage", &conf, "dev-uk".into()).unwrap();
        // verify datahandling implicits
        let dh = mf.dataHandling.unwrap();
        let s3 = dh.stores[0].clone();
        assert!(s3.encrypted.unwrap());
        assert_eq!(s3.fields[0].encrypted.unwrap(), false); // overridden
        assert_eq!(s3.fields[1].encrypted.unwrap(), true); // cascaded
        assert_eq!(s3.fields[0].keyRotator, None); // not set either place
        assert_eq!(s3.fields[1].keyRotator, Some("2w".into())); // field value
    }

    #[test]
    fn templating_test() {
        setup();
        let conf = Config::read().unwrap();
        let mf = Manifest::completed("fake-ask", &conf, "dev-uk".into()).unwrap();

        // verify templating
        let env = mf.env;
        assert_eq!(env["CORE_URL"], "https://woot.com/somesvc".to_string());
        // check values from Config - one plain, one as_secret
        assert_eq!(env["CLIENT_ID"], "FAKEASKID".to_string());
        assert!(env.get("CLIENT_SECRET").is_none()); // moved to secret
        let sec = mf.secrets;
        assert_eq!(sec["CLIENT_SECRET"], "FAKEASKSECRET".to_string()); // via reg.kong consumers
        assert_eq!(sec["FAKE_SECRET"], "hello".to_string()); // NB: ACTUALLY IN_VAULT

        let configs = mf.configs.clone().unwrap();
        let configini = configs.files[0].clone();
        let cfgtpl = configini.value.unwrap();
        print!("{:?}", cfgtpl);
        assert!(cfgtpl.contains("CORE=https://woot.com/somesvc"));
        assert!(cfgtpl.contains("CLIENT_ID"));
        assert!(cfgtpl.contains("CLIENT_ID=FAKEASKID"));
    }
}
