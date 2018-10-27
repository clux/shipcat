use std::collections::BTreeMap;
//use serde_yaml;
use regex::Regex;


use super::{Result, Config};

// All structs come from the structs directory
use super::structs::traits::Verify;
use super::structs::{HealthCheck, ConfigMap};
use super::structs::{InitContainer, Resources, HostAlias};
use super::structs::volume::{Volume, VolumeMount};
use super::structs::{Metadata, VaultOpts, Dependency};
use super::structs::security::DataHandling;
use super::structs::Probe;
use super::structs::{CronJob, Sidecar};
use super::structs::{Kafka, Kong, Rbac};
use super::structs::RollingUpdate;
use super::structs::autoscaling::AutoScaling;
use super::structs::tolerations::Tolerations;
use super::structs::LifeCycle;
use super::structs::Worker;
use super::structs::Port;

/// Main manifest, serializable from shipcat.yml
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    // ------------------------------------------------------------------------
    // !! special properties first !!
    //
    // Manifest syntax extras should be added AFTER all these.
    // These special properties, that are generally not overrideable, and should be
    // marked with `skip_deserializing` serde field attributes.
    // ------------------------------------------------------------------------

    /// Name of the service
    ///
    /// This must match the folder name in a manifests repository.
    #[serde(default)]
    pub name: String,

    /// Region injected into helm chart
    ///
    /// Exposed from shipcat, but not overrideable.
    #[serde(default, skip_deserializing)]
    pub region: String,

    /// Environment injected into the helm chart
    ///
    /// Exposed from shipcat, but not overrideable.
    #[serde(default, skip_deserializing)]
    pub environment: String,

    /// Namespace injected in helm chart
    ///
    /// Exposed from shipcat, but not overrideable.
    #[serde(default, skip_deserializing)]
    pub namespace: String,

    /// Regions to deploy this service to.
    ///
    /// Every region must be listed in here.
    /// Uncommenting a region in here will partially disable this service.
    #[serde(default, skip_serializing)]
    pub regions: Vec<String>,

    /// Wheter the service should be public
    ///
    /// This is a special flag not exposed to the charts at the moment.
    #[serde(default, skip_serializing)]
    pub publiclyAccessible: bool,

    // Decoded secrets - only used interally
    #[serde(default, skip_serializing, skip_deserializing)]
    pub _decoded_secrets: BTreeMap<String, String>,

    /// Service is disabled
    ///
    /// This disallows usage of this service in all regions.
    #[serde(default, skip_serializing)]
    pub disabled: bool,

    /// Service is external
    ///
    /// This cancels all validation and marks the manifest as a non-kube reference only.
    #[serde(default, skip_serializing)]
    pub external: bool,

    /// Raw secrets from environment variables.
    ///
    /// The `env` map fills in secrets in this via the `vault` client.
    /// `Manifest::secrets` partitions `env` into `env` and `secrets`.
    /// See `Manifest::env`.
    ///
    /// This means that this is an internal property that is actually exposed!
    #[serde(default, skip_deserializing, skip_serializing_if = "BTreeMap::is_empty")]
    pub secrets: BTreeMap<String, String>,


    // ------------------------------------------------------------------------
    // mergeable properties below
    //
    // BEFORE ADDING PROPERTIES READ THIS
    // Below are properties that can be merged, above ones that are global
    // if you add anything below here, also add it to merge.rs!
    // ------------------------------------------------------------------------


    /// Chart to use for the service
    ///
    /// All the properties in `Manifest` are tailored towards our `base` chart,
    /// so this should be overridden with caution.
    #[serde(default)]
    pub chart: Option<String>,

    /// Image name of the docker image to run
    ///
    /// This can be left out if imagePrefix is set in the config, and the image name
    /// also matches the service name. Otherwise, this needs to be the full image name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    /// Optional uncompressed image size
    ///
    /// This is used to compute a more accurate wait time for rolling upgrades.
    /// See `Manifest::estimate_wait_time`.
    #[serde(skip_serializing)]
    pub imageSize: Option<u32>,

    /// Version aka. tag of docker image to run
    ///
    /// This does not have to be set in "rolling environments", where upgrades
    /// re-use the current running versions. However, for complete control, production
    /// environments should put the versions in manifests.
    ///
    /// Versions must satisfy `VersionScheme,::verify`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Command to use for the docker image
    ///
    /// This can be left out to use the default image command.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,


    /// Important contacts and other metadata for the service
    ///
    /// Particular uses:
    /// - notifying correct people on upgrades via slack
    /// - providing direct links to code diffs on upgrades in slack
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    /// Data sources and handling strategies
    ///
    /// An experimental abstraction around GDPR
    #[serde(default, skip_serializing)]
    pub dataHandling: Option<DataHandling>,

    /// Language the service is written in
    ///
    /// This does not provide any special behaviour at the moment.
    #[serde(skip_serializing)]
    pub language: Option<String>,


    /// Kubernetes resource limits and requests
    ///
    /// Api straight from https://kubernetes.io/docs/concepts/configuration/manage-compute-resources-container/
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Resources<String>>,

    /// Kubernetes replication count
    ///
    /// This is set on the `Deployment` object in kubernetes.
    /// If you have `autoScaling` parameters set, then these take precedence.
    #[serde(default)]
    pub replicaCount: Option<u32>,


    /// Environment variables to inject
    ///
    /// These have a few special convenience behaviours:
    /// "IN_VAULT" values is replaced with value from vault/secret/folder/service/KEY
    /// One off `tera` templates are calculated with a limited template context
    ///
    /// IN_VAULT secrets will all be put in a single kubernetes `Secret` object.
    /// One off templates **can** be put in a `Secret` object if marked `| as_secret`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,


    /// Kubernetes Secret Files to inject
    ///
    /// These have the same special "IN_VAULT" behavior as `Manifest::env`:
    /// "IN_VAULT" values is replaced with value from vault/secret/folder/service/key
    ///
    /// Note the lowercase restriction on keys.
    /// All `secretFiles` are expected to be base64 in vault, and are placed into a
    /// kubernetes `Secret` object.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secretFiles: BTreeMap<String, String>,


    /// Config files to inline in a kubernetes `ConfigMap`
    ///
    /// These are read and templated by `tera` before they are passed to helm.
    /// A full `tera` context from `Manifest::make_template_context` is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configs: Option<ConfigMap>,

    /// Vault options
    ///
    /// Allows overriding service names and regions for secrets.
    /// DEPRECATED. Should only be set in rare cases.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault: Option<VaultOpts>,

    /// Http Port to expose in the kubernetes `Service`
    ///
    /// This is normally the service your application listens on.
    /// Kong deals with mapping the port to a nicer one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub httpPort: Option<u32>,

    /// Ports to open
    ///
    /// For services outside Kong, expose these named ports in the kubernetes `Service`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<Port>,

    /// Externally exposed port
    ///
    /// Useful for `LoadBalancer` type `Service` objects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub externalPort: Option<u32>,

    /// Health check parameters
    ///
    /// A small abstraction around `readinessProbe`.
    /// DEPRECATED. Should use `readinessProbe`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<HealthCheck>,

    /// Service dependencies
    ///
    /// Used to construct a dependency graph, and in the case of non-circular trees,
    /// it can be used to arrange deploys in the correct order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Dependency>,

    /// Worker `Deployment` objects to additinally include
    ///
    /// These are more flexible than `sidecars`, because they scale independently of
    /// the main `replicaCount`. However, they are considered separate rolling upgrades.
    /// There is no guarantee that these switch over at the same time as your main
    /// kubernetes `Deployment`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workers: Vec<Worker>,

    /// Sidecars to inject into every kubernetes `Deployment`
    ///
    /// Plain sidecars are injected into the main `Deployment` and all the workers' ones.
    /// They scale directly with the sum of `replicaCount`s.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sidecars: Vec<Sidecar>,

    /// `readinessProbe` for kubernetes
    ///
    /// This configures the service's health check, which is used to gate rolling upgrades.
    /// https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-probes/
    ///
    /// This replaces shipcat's `Manifest::health` abstraction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readinessProbe: Option<Probe>,

    /// `livenessProbe` for kubernetes
    ///
    /// This configures a `readinessProbe` check, with the instruction to kill on failure.
    /// https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-probes/
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub livenessProbe: Option<Probe>,

    /// Container lifecycle events for kubernetes
    ///
    /// This allows commands to be executed either `postStart` or `preStop`
    /// https://kubernetes.io/docs/tasks/configure-pod-container/attach-handler-lifecycle-event/
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<LifeCycle>,

    /// Rolling update Deployment parameters
    ///
    /// These tweak the speed and care kubernetes uses when doing a rolling update.
    /// https://kubernetes.io/docs/concepts/workloads/controllers/deployment/#rolling-update-deployment
    /// This is attached onto the main `Deployment`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollingUpdate: Option<RollingUpdate>,

    /// `HorizontalPodAutoScaler` parameters for kubernetes
    ///
    /// Passed all parameters directly onto a `spec` of a kube HPA.
    /// https://kubernetes.io/docs/tasks/run-application/horizontal-pod-autoscale/
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoScaling: Option<AutoScaling>,

    /// Toleration parameters for kubernetes
    ///
    /// Bind a service to a particular type of kube `Node`.
    /// https://kubernetes.io/docs/concepts/configuration/taint-and-toleration/
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tolerations: Vec<Tolerations>,

    /// Host aliases to inject in /etc/hosts in every kubernetes `Pod`
    ///
    /// https://kubernetes.io/docs/concepts/services-networking/add-entries-to-pod-etc-hosts-with-host-aliases/
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hostAliases: Vec<HostAlias>,

    /// `initContainer` list for every kubernetes `Pod`
    ///
    /// Allows database connectivity checks to be done as pre-boot init-step.
    /// https://kubernetes.io/docs/concepts/workloads/pods/init-containers/
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub initContainers: Vec<InitContainer>,

    /// Volumes that can be mounted in every kubernetes `Pod`
    ///
    /// Supports our subset of: https://kubernetes.io/docs/concepts/storage/volumes/
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumes: Vec<Volume>,

    /// Volumes to mount to every kubernetes `Pod`
    ///
    /// Requires the `Manifest::volumes` entries.
    /// https://kubernetes.io/docs/concepts/storage/volumes/
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumeMounts: Vec<VolumeMount>,


    /// Cronjobs images to run as kubernetes `CronJob` objects
    ///
    /// Limited usefulness abstraction, that should be avoided.
    /// https://kubernetes.io/docs/concepts/workloads/controllers/cron-jobs/
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cronJobs: Vec<CronJob>,

    /// Annotations to set on `Service` objects
    ///
    /// Useful for `LoadBalancer` type `Service` objects.
    /// Not useful for kong balanced services.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub serviceAnnotations: BTreeMap<String, String>,

    /// Labels for every kubernetes object
    ///
    /// Injected in all top-level kubernetes object as a prometheus convenience.
    /// https://kubernetes.io/docs/concepts/overview/working-with-objects/labels/
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,

    /// Kong config
    ///
    /// A mostly straight from API configuration struct for Kong
    /// Work in progress. `structs::kongfig` contain the newer abstractions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kong: Option<Kong>,
    /// Hosts to override kong hosts
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hosts: Vec<String>,

    /// Kafka config
    ///
    /// A small convencience struct to indicate that the service uses `Kafka`.
    /// The chart will inject a few environment variables and a kafka initContainer
    /// if this is set to a `Some`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kafka: Option<Kafka>,

    /// Load balancer source ranges
    ///
    /// This is useful for charts that expose a `Service` of `LoadBalancer` type.
    /// IP CIDR ranges, which Kubernetes will use to configure firewall exceptions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sourceRanges: Vec<String>,

    /// Role-Based Access Control
    ///
    /// A list of resources to allow the service access to use.
    /// This is a subset of kubernetes `Role::rules` parameters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rbac: Vec<Rbac>,
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
