use std::collections::BTreeMap;
use serde_yaml;
use regex::Regex;


use super::{Result, Config};

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
use super::structs::{Kafka, Kong, Rbac};
use super::structs::RollingUpdate;
use super::structs::autoscaling::AutoScaling;
use super::structs::tolerations::Tolerations;
use super::structs::Worker;
use super::structs::Port;

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
    pub resources: Option<Resources<String>>,
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
    /// Ports to open
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<Port>,
    /// Externally exposed port
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub externalPort: Option<u32>,
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
    /// Hosts to override kong hosts
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hosts: Vec<String>,

    /// Kafka config
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kafka: Option<Kafka>,

    /// Kube Secret Files to append
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secretFiles: BTreeMap<String, String>,

    /// Load balancer source ranges
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sourceRanges: Vec<String>,

    /// Role-Based Access Control
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rbac: Vec<Rbac>,

    // TODO: logging alerts
}

impl Manifest {
    /// Override version with an optional one from the CLI
    pub fn set_version(mut self, ver: &Option<String>) -> Self {
        if ver.is_some() {
            self.version = ver.clone(); // override version here if set
        }
        self
    }

    /// Print manifest to stdout
    pub fn print(&self) -> Result<()> {
        print!("{}\n", serde_yaml::to_string(self)?);
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
        for p in &self.ports {
            p.verify(&conf)?;
        }
        for r in &self.rbac {
            r.verify(&conf)?;
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
            if k.contains("-") {
                bail!("Env vars need to use SCREAMING_SNAKE_CASE not dashes, found: {}", k);
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
