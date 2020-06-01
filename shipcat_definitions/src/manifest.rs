use crate::vault::Vault;
use kube_derive::CustomResource;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};

use super::Result;
use crate::{
    config::Config,
    region::{Region, VaultConfig},
    states::{ManifestState, PrimaryWorkload},
    ManifestStatus,
};

// All structs come from the structs directory
use super::structs::{
    autoscaling::AutoScaling,
    newrelic::Newrelic,
    security::DataHandling,
    sentry::Sentry,
    tolerations::Tolerations,
    volume::{Volume, VolumeMount},
    ConfigMap, Container, CronJob, Dependency, DestinationRule, EnvVars, EventStream, Gate, HealthCheck,
    HostAlias, Kafka, KafkaResources, Kong, LifeCycle, Metadata, NotificationMode, PersistentVolume, Port,
    Probe, PrometheusAlert, Rbac, ResourceRequirements, RollingUpdate, SecurityContext, VaultOpts, Worker,
};

/// Main manifest, serializable from manifest.yml or the shipcat CRD.
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, Default)]
#[kube(
    group = "babylontech.co.uk",
    kind = "ShipcatManifest",
    version = "v1",
    namespaced,
    shortname = "sm",
    status = "ManifestStatus",
    printcolumn = r#"{"name":"Kong", "jsonPath": ".spec.kong_apis[*].uris", "type": "string", "description": "The URI where the service is available through kong"}"#,
    printcolumn = r#"{"name":"Version", "jsonPath": ".spec.version", "type": "string", "description": "The version of the service that is deployed"}"#,
    printcolumn = r#"{"name":"Team", "jsonPath": ".spec.metadata.team", "type": "string", "description": "The team that owns the service"}"#
)]
#[kube(apiextensions = "v1beta1")] // kubernetes < 1.16
pub struct Manifest {
    // ------------------------------------------------------------------------
    // Non-mergeable global properties
    //
    // A few global properties that cannot be overridden in region override manifests.
    // These are often not exposed to kube and marked with `skip_serializing`,
    // but more often data that is used internally and assumed global.
    // ------------------------------------------------------------------------
    /// Name of the service
    ///
    /// This must match the folder name in a manifests repository, and additionally;
    /// - length limits imposed by kube dns
    /// - dash separated, alpha numeric names (for dns readability)
    ///
    /// The main validation regex is: `^[0-9a-z\-]{1,50}$`.
    ///
    /// ```yaml
    /// name: webapp
    /// ```
    #[serde(default)]
    pub name: String,

    /// Whether the service should be public
    ///
    /// This is a special flag not exposed to the charts at the moment.
    ///
    /// ```yaml
    /// publiclyAccessible: true
    /// ```
    #[serde(default, skip_serializing)]
    pub publiclyAccessible: bool,

    /// Service is external
    ///
    /// This cancels all validation and marks the manifest as a non-kube reference only.
    ///
    /// ```yaml
    /// external: true
    /// ```
    #[serde(default, skip_serializing)]
    pub external: bool,

    /// Service is disabled
    ///
    /// This disallows usage of this service in all regions.
    ///
    /// ```yaml
    /// disabled: true
    /// ```
    #[serde(default, skip_serializing)]
    pub disabled: bool,

    /// Regions to deploy this service to.
    ///
    /// Every region must be listed in here.
    /// Uncommenting a region in here will partially disable this service.
    #[serde(default, skip_serializing)]
    pub regions: Vec<String>,

    /// Important contacts and other metadata for the service
    ///
    /// Particular uses:
    /// - notifying correct people on upgrades via slack
    /// - providing direct links to code diffs on upgrades in slack
    ///
    /// ```yaml
    /// metadata:
    ///   contacts:
    ///   - name: "Eirik"
    ///     slack: "@clux"
    ///   team: Doves
    ///   repo: https://github.com/clux/blog
    ///   support: "#humans"
    ///   notifications: "#robots"
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    // ------------------------------------------------------------------------
    // Regular mergeable properties
    //
    // New syntax goes in here!
    // All properties in here should be mergeable, so ensure you add merge behaviour.
    // Merge behaviour is defined in the merge module.
    // ------------------------------------------------------------------------
    /// Chart to use for the service
    ///
    /// All the properties in `Manifest` are tailored towards our `base` chart,
    /// so this should be overridden with caution.
    ///
    /// ```yaml
    /// chart: custom
    /// ```
    #[serde(default)]
    pub chart: Option<String>,

    /// Image name of the docker image to run
    ///
    /// This can be left out if imagePrefix is set in the config, and the image name
    /// also matches the service name. Otherwise, this needs to be the full image name.
    ///
    /// ```yaml
    /// image: nginx
    /// ```
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    /// Optional uncompressed image size
    ///
    /// This is used to compute a more accurate wait time for rolling upgrades.
    /// See `Manifest::estimate_wait_time`.
    ///
    /// Ideally, this number is autogenerated from your docker registry.
    ///
    /// ```yaml
    /// imageSize: 1400
    /// ```
    #[serde(skip_serializing)]
    pub imageSize: Option<u32>,

    /// Version aka. tag of docker image to run
    ///
    /// This does not have to be set in "rolling environments", where upgrades
    /// re-use the current running versions. However, for complete control, production
    /// environments should put the versions in manifests.
    ///
    /// Versions must satisfy `VersionScheme::verify`.
    ///
    ///
    /// ```yaml
    /// version: 1.2.0
    /// ```
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Command to use for the docker image
    ///
    /// This can be left out to use the default image command.
    ///
    /// ```yaml
    /// command: ["bundle", "exec", "rake", "jobs:work"]
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,

    /// Extend the workload with a securityContext
    ///
    /// This allows changing the ownership of mounted volumes
    ///
    /// ```yaml
    /// securityContext:
    ///   runAsUser: 1000
    ///   fsGroup: 1000
    /// ```
    #[serde(skip_serializing_if = "Option::is_none")]
    pub securityContext: Option<SecurityContext>,

    /// Data sources and handling strategies
    ///
    /// An experimental abstraction around GDPR
    ///
    /// ```yaml
    /// dataHandling:
    ///   stores:
    ///   - backend: Postgres
    ///     encrypted: true
    ///     cipher: AES256
    ///     fields:
    ///     - name: BabylonUserId
    ///     - name: HealthCheck
    ///   processes:
    ///   - field: HealthCheck
    ///     source: orchestrator
    /// ```
    #[serde(default, skip_serializing)]
    pub dataHandling: Option<DataHandling>,

    /// Kubernetes resource limits and requests
    ///
    /// Api straight from [kubernetes resources](https://kubernetes.io/docs/concepts/configuration/manage-compute-resources-container/)
    ///
    /// ```yaml
    /// resources:
    ///   requests:
    ///     cpu: 100m
    ///     memory: 100Mi
    ///   limits:
    ///     cpu: 300m
    ///     memory: 300Mi
    /// ```
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceRequirements<String>>,

    /// Kubernetes replication count
    ///
    /// This is set on the `Deployment` object in kubernetes.
    /// If you have `autoScaling` parameters set, then these take precedence.
    ///
    /// ```yaml
    /// replicaCount: 4
    /// ```
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
    ///
    /// ```yaml
    /// env:
    ///   # plain eva:
    ///   RUST_LOG: "tokio=info,raftcat=debug"
    ///
    ///   # vault lookup:
    ///   DATABASE_URL: IN_VAULT
    ///
    ///   # templated evars:
    ///   INTERNAL_AUTH_URL: "{{ base_urls.services }}/auth/internal"
    ///   REGION_NAME: "{{ region }}"
    ///   NAMESPACE: "{{ namespace }}"

    /// ```
    ///
    /// The vault lookup will GET from the region specific path for vault, in the
    /// webapp subfolder, getting the `DATABASE_URL` secret.
    #[serde(default)]
    pub env: EnvVars,

    /// Kubernetes Secret Files to inject
    ///
    /// These have the same special "IN_VAULT" behavior as `Manifest::env`:
    /// "IN_VAULT" values is replaced with value from vault/secret/folder/service/key
    ///
    /// Note the lowercase restriction on keys.
    /// All `secretFiles` are expected to be base64 in vault, and are placed into a
    /// kubernetes `Secret` object.
    ///
    /// ```yaml
    /// secretFiles:
    ///   webapp-ssl-keystore: IN_VAULT
    ///   webapp-ssl-truststore: IN_VAULT
    /// ```
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secretFiles: BTreeMap<String, String>,

    /// Config files to inline in a kubernetes `ConfigMap`
    ///
    /// These are read and templated by `tera` before they are passed to helm.
    /// A full `tera` context from `Manifest::make_template_context` is used.
    ///
    /// ```yaml
    /// configs:
    ///   mount: /config/
    ///   files:
    ///   - name: webhooks.json.j2
    ///     dest: webhooks.json
    ///   - name: newrelic-java.yml.j2
    /// ```
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
    /// ```yaml
    /// httpPort: 8000
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub httpPort: Option<u32>,

    /// Ports to open
    ///
    /// For services outside Kong, expose these named ports in the kubernetes `Service`.
    ///
    /// ```yaml
    ///  ports:
    ///  - port: 6121
    ///    name: data
    ///  - port: 6122
    ///    name: rpc
    ///  - port: 6125
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<Port>,

    /// Externally exposed port
    ///
    /// Useful for `LoadBalancer` type `Service` objects.
    ///
    /// ```yaml
    /// externalPort: 443
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub externalPort: Option<u32>,

    /// Health check parameters
    ///
    /// A small abstraction around `readinessProbe`.
    /// DEPRECATED. Should use `readinessProbe`.
    ///
    /// ```yaml
    /// health:
    ///   uri: /health
    ///   wait: 15
    /// ```
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<HealthCheck>,

    /// Service dependencies
    ///
    /// Used to construct a dependency graph, and in the case of non-circular trees,
    /// it can be used to arrange deploys in the correct order.
    ///
    /// ```yaml
    /// dependencies:
    /// - name: auth
    /// - name: ask2
    /// - name: chatbot-reporting
    /// - name: clinical-knowledge
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Dependency>,

    /// Destination Rules
    ///
    /// The intention here is that implementations will examine requests to determine if they
    /// satisfy this rule and if so, redirect them to alternative services as specified by 'host'.
    ///
    /// For an example, one could implement destination rules using an Istio virtual service
    /// which matched on inbound request header values to determine whether to apply this rule and
    /// redirect the request.
    ///
    /// ```yaml
    /// destinationRules:
    /// - identifier: 'USA'
    ///   host: 'service.com'
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destinationRules: Option<Vec<DestinationRule>>,

    /// Worker `Deployment` objects to additionally include
    ///
    /// These are more flexible than `sidecars`, because they scale independently of
    /// the main `replicaCount`. However, they are considered separate rolling upgrades.
    /// There is no guarantee that these switch over at the same time as your main
    /// kubernetes `Deployment`.
    ///
    /// ```yaml
    /// workers:
    /// - name: analytics-experiment-taskmanager
    ///   resources:
    ///     limits:
    ///       cpu: 1
    ///       memory: 1Gi
    ///     requests:
    ///       cpu: 250m
    ///       memory: 1Gi
    ///   replicaCount: 3
    ///   preserveEnv: true
    ///   ports:
    ///   - port: 6121
    ///     name: data
    ///   - port: 6122
    ///     name: rpc
    ///   - port: 6125
    ///     name: query
    ///   command: ["/start.sh", "task-manager", "-Djobmanager.rpc.address=analytics-experiment"]
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workers: Vec<Worker>,

    /// Sidecars to inject into every kubernetes `Deployment`
    ///
    /// Plain sidecars are injected into the main `Deployment` and all the workers' ones.
    /// They scale directly with the sum of `replicaCount`s.
    ///
    /// ```yaml
    /// sidecars:
    /// - name: redis
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sidecars: Vec<Container>,

    /// `readinessProbe` for kubernetes
    ///
    /// This configures the service's health check, which is used to gate rolling upgrades.
    /// Api is a direct translation of [kubernetes liveness/readiness probes](https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-probes/).
    ///
    /// This replaces shipcat's `Manifest::health` abstraction.
    ///
    /// ```yaml
    /// readinessProbe:
    ///   httpGet:
    ///     path: /
    ///     port: http
    ///     httpHeaders:
    ///     - name: X-Forwarded-Proto
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readinessProbe: Option<Probe>,

    /// `livenessProbe` for kubernetes
    ///
    /// This configures a `livenessProbe` check. Similar to `readinessProbe`, but with the instruction to kill the pod on failure.
    /// Api is a direct translation of [kubernetes liveness/readiness probes](https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-probes/).
    ///
    /// ```yaml
    /// livenessProbe:
    ///   tcpSocket:
    ///     port: redis
    ///   initialDelaySeconds: 15
    ///   periodSeconds: 15
    /// ```
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
    /// Sraight from [kubernetes rolling update parameters](https://kubernetes.io/docs/concepts/workloads/controllers/deployment/#rolling-update-deployment).
    /// This is attached onto the main `Deployment`.
    ///
    /// ```yaml
    /// rollingUpdate:
    ///   maxUnavailable: 0%
    ///   maxSurge: 50%
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollingUpdate: Option<RollingUpdate>,

    /// `HorizontalPodAutoScaler` parameters for kubernetes
    ///
    /// Passed all parameters directly onto the `spec` of a kube HPA.
    /// Straight from [kubernetes horizontal pod autoscaler](https://kubernetes.io/docs/tasks/run-application/horizontal-pod-autoscale/).
    ///
    /// ```yaml
    /// autoScaling:
    ///   minReplicas: 6
    ///   maxReplicas: 9
    ///   metrics:
    ///   - type: Resource
    ///     resource:
    ///       name: cpu
    ///       targetAverageUtilization: 60
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoScaling: Option<AutoScaling>,

    /// Toleration parameters for kubernetes
    ///
    /// Bind a service to a particular type of kube `Node`.
    /// Straight from [kubernetes taints and tolerations](https://kubernetes.io/docs/concepts/configuration/taint-and-toleration/).
    ///
    /// ```yaml
    /// tolerations:
    /// - key: "dedicated"
    ///   operator: "Equal"
    ///   value: "hugenode"
    ///   effect: "NoSchedule"
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tolerations: Vec<Tolerations>,

    /// Host aliases to inject in /etc/hosts in every kubernetes `Pod`
    ///
    /// Straight from [kubernetes host aliases](https://kubernetes.io/docs/concepts/services-networking/add-entries-to-pod-etc-hosts-with-host-aliases/).
    ///
    /// ```yaml
    /// hostAliases:
    /// - ip: "160.160.160.160"
    ///   hostnames:
    ///   - weird-service.babylontech.co.uk
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hostAliases: Vec<HostAlias>,

    /// `initContainer` list for every kubernetes `Pod`
    ///
    /// Allows database connectivity checks to be done as pre-boot init-step.
    /// Straight frok [kubernetes init containers](https://kubernetes.io/docs/concepts/workloads/pods/init-containers/).
    ///
    /// ```yaml
    /// initContainers:
    /// - name: init-cassandra
    ///   image: gophernet/netcat
    ///   command: ['sh', '-c', 'until nc -z dev-cassandra 9042; do sleep 2; done;']
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub initContainers: Vec<Container>,

    /// Volumes that can be mounted in every kubernetes `Pod`
    ///
    /// Supports our subset of [kubernetes volumes](https://kubernetes.io/docs/concepts/storage/volumes/)
    ///
    /// ```yaml
    /// volumes:
    /// - name: google-creds
    ///   secret:
    ///     secretName: google-creds
    ///     items:
    ///     - key: file
    ///       path: google-cloud-creds.json
    ///       mode: 0o777
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumes: Vec<Volume>,

    /// Volumes to mount to every kubernetes `Pod`
    ///
    /// Requires the `Manifest::volumes` entries.
    /// Straight from [kubernetes volumes](https://kubernetes.io/docs/concepts/storage/volumes/)
    ///
    /// ```yaml
    /// volumeMounts:
    /// - name: ssl-store-files
    ///   mountPath: /conf/ssl/
    ///   readOnly: true
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumeMounts: Vec<VolumeMount>,

    /// PersistentVolumes for the deployment
    ///
    /// Exposed from shipcat, but not overrideable.
    /// Mostly straight from [kubernetes persistent volumes](https://kubernetes.io/docs/concepts/storage/persistent-volumes).
    ///
    /// ```yaml
    /// persistentVolumes:
    /// - name: svc-cache-space
    ///   mountPath: /root/.scratch
    ///   size: 10Gi
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub persistentVolumes: Vec<PersistentVolume>,

    /// Cronjob images to run as kubernetes `CronJob` objects
    ///
    /// Limited usefulness abstraction, that should be avoided.
    /// An abstraction on top of [kubernetes cron jobs](https://kubernetes.io/docs/concepts/workloads/controllers/cron-jobs/)
    ///
    /// ```yaml
    /// cronJobs:
    /// - name: webapp-promotions-expire
    ///   schedule: "1 0 * * *"
    ///   command: ["bundle", "exec", "rake", "cron:promotions:expire", "--silent"]
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cronJobs: Vec<CronJob>,

    /// Annotations to set on `Service` objects
    ///
    /// Useful for `LoadBalancer` type `Service` objects.
    /// Not useful for kong balanced services.
    ///
    /// ```yaml
    /// serviceAnnotations:
    ///   svc.k8s.io/aws-load-balancer-ssl-cert: arn:aws:acm:eu-west-2:12345:certificate/zzzz
    ///   svc.k8s.io/aws-load-balancer-backend-protocol: http
    ///   svc.k8s.io/aws-load-balancer-ssl-ports: "443"
    ///   svc.k8s.io/aws-load-balancer-ssl-negotiation-policy: ELBSecurityPolicy-TLS-1-2-2018-01
    ///   helm.sh/resource-policy: keep
    /// ```
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub serviceAnnotations: BTreeMap<String, String>,

    /// Metadata Annotations for pod spec templates in deployments, and cron jobs
    ///
    /// https://kubernetes.io/docs/concepts/overview/working-with-objects/annotations/
    ///
    /// ```yaml
    /// podAnnotations:
    ///   iam.amazonaws.com/role: role-arn
    /// ```
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub podAnnotations: BTreeMap<String, String>,

    /// Labels for every kubernetes object
    ///
    /// Injected in all top-level kubernetes object as a prometheus convenience.
    /// https://kubernetes.io/docs/concepts/overview/working-with-objects/labels/
    ///
    /// ```yaml
    /// labels:
    ///   custom-metrics: true
    /// ```
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,

    /// Kong config
    ///
    /// A mostly straight from API configuration struct for Kong
    /// Work in progress. `structs::kongfig` contain the newer abstractions.
    ///
    /// ```yaml
    /// kong:
    ///   uris: /webapp
    ///   strip_uri: true
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kongApis: Vec<Kong>,

    ///  Deprecated Gate config
    ///
    /// Do not use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<Gate>,

    /// Kafka config
    ///
    /// A small convencience struct to indicate that the service uses `Kafka`,
    /// and to define kafka-specific properties.
    /// if this is set to a `Some`.
    ///
    /// ```yaml
    /// kafka: {}
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kafka: Option<Kafka>,

    /// Load balancer source ranges
    ///
    /// This is useful for charts that expose a `Service` of `LoadBalancer` type.
    /// IP CIDR ranges, which Kubernetes will use to configure firewall exceptions.
    ///
    /// ```yaml
    /// sourceRanges:
    /// - 0.0.0.0/0
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sourceRanges: Vec<String>,

    /// Role-Based Access Control
    ///
    /// A list of resources to allow the service access to use.
    /// This is a subset of kubernetes `Role::rules` parameters.
    ///
    /// ```yaml
    /// rbac:
    /// - apiGroups: ["extensions"]
    ///   resources: ["deployments"]
    ///   verbs: ["get", "watch", "list"]
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rbac: Vec<Rbac>,

    /// Kafka / EventStream configuration
    ///
    /// A list of resources that will interact with the Kafka-operator CRD /
    /// service to create kafka topics and ACLs. The Kafka-Operator is an
    /// extension of the strimzi-kafka-operator project:
    /// - https://strimzi.io/
    /// - https://github.com/strimzi/strimzi-kafka-operator
    ///
    ///
    /// ```yaml
    ///  eventStreams:
    ///  - name: topicA
    ///    producers:
    ///    - service1
    ///    - service2
    ///    consumers:
    ///    - service3
    ///    - service4
    ///    eventDefinitions:
    ///    - key: my_schema_key
    ///      value: my_schema_value
    ///    - key: my_schema_key_1
    ///      value: my_schema_value_1
    ///    config:
    ///        retention.ms: "7200000"
    ///        segment.bytes: "1073741824"
    /// ```

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub eventStreams: Vec<EventStream>,

    /// Kafka Resources (Topics and Users)
    ///
    /// inputs for this struct relate directly to the strimzi kafka project.
    /// Topic Inputs: [Strimzi Kafka Topic CRD ](https://github.com/strimzi/strimzi-kafka-operator/blob/master/install/topic-operator/04-Crd-kafkatopic.yaml)
    /// User Inputs: [Strimzi Kafka User CRD ](https://github.com/strimzi/strimzi-kafka-operator/blob/master/install/user-operator/04-Crd-kafkauser.yaml)
    ///
    /// ```yaml
    /// kafkaResources:
    ///   topics:
    ///   - name: foo-topic-name
    ///     partitions: 1
    ///     replicas: 3
    ///     config:
    ///       retention.ms: 604800000
    ///       segment.bytes: 1073741824
    ///   users:
    ///   - name: foo-user-name
    ///     acls:
    ///     - resourceName: testtopic
    ///       resourceType: topic
    ///       patternType: literal
    ///       operation: write
    ///       host: "*"
    ///     - resourceName: testtopic
    ///       resourceType: topic
    ///       patternType: literal
    ///       operation: read
    ///       host: "*"
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kafkaResources: Option<KafkaResources>,

    /// Monitoring section covering NewRelic configuration
    ///
    /// ```yaml
    /// newrelic:
    ///   alerts:
    ///     alert_name_foo:
    ///       name: alert_name_foo:
    ///       template: appdex
    ///       params:
    ///         threshold: "0.5"
    ///         priority: critical
    ///   incidentPreference: PER_POLICY
    ///   slack: C12ABYZ78
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub newrelic: Option<Newrelic>,

    /// Monitoring section covering Sentry configuration
    ///
    /// ```yaml
    /// sentry:
    ///   slack: C12ABYZ78
    ///   silent: false
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sentry: Option<Sentry>,

    /// Slack upgrade notification settings
    ///
    /// ```yaml
    /// upgradeNotifications: Silent
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upgradeNotifications: Option<NotificationMode>,

    // ------------------------------------------------------------------------
    // Output variables
    //
    // Properties here cannot be deserialized and are illegal in manifests!
    // if you add anything below here, you need to handle behaviour for it.
    // These must be marked with `skip_deserializing` serde attributes.
    // ------------------------------------------------------------------------
    /// Region injected into helm chart
    ///
    /// Exposed from shipcat, but not overrideable.
    #[serde(default)]
    #[cfg_attr(feature = "filesystem", serde(skip_deserializing))]
    pub region: String,

    /// Environment injected into the helm chart
    ///
    /// Exposed from shipcat, but not overrideable.
    #[serde(default)]
    #[cfg_attr(feature = "filesystem", serde(skip_deserializing))]
    pub environment: String,

    /// Namespace injected in helm chart
    ///
    /// Exposed from shipcat, but not overrideable.
    #[serde(default)]
    #[cfg_attr(feature = "filesystem", serde(skip_deserializing))]
    pub namespace: String,

    /// Uid from the CRD injected into the helm chart
    ///
    /// This is required to inject into the charts due to
    /// https://github.com/kubernetes/kubernetes/issues/66068
    ///
    /// Exposed from shipcat, but not overrideable.
    #[serde(default)]
    #[cfg_attr(
        feature = "filesystem",
        serde(skip_deserializing, skip_serializing_if = "Option::is_none")
    )]
    pub uid: Option<String>,

    /// Raw secrets from environment variables.
    ///
    /// The `env` map fills in secrets in this via the `vault` client.
    /// `Manifest::secrets` partitions `env` into `env` and `secrets`.
    /// See `Manifest::env`.
    ///
    /// This is an internal property that is exposed as an output only.
    #[serde(default, skip_deserializing, skip_serializing_if = "BTreeMap::is_empty")]
    pub secrets: BTreeMap<String, String>,

    /// Internal state of the manifest
    ///
    /// A manifest goes through different stages of serialization, templating,
    /// config loading, secret injection. This property keeps track of it.
    #[serde(default, skip_deserializing, skip_serializing)]
    pub state: ManifestState,

    /// The default workload associated with a Manifest
    ///
    /// Defaults to Deployment
    ///
    /// ```yaml
    /// workload: Statefulset
    /// ```
    #[serde(default)]
    pub workload: PrimaryWorkload,

    /// Prometheus alerts associated with the service.
    ///
    /// ```yaml
    /// prometheusAlerts:
    /// - name: AlertNameInPascalCase
    ///   summary: "One-line summary of the issue"
    ///   description: "More details about the issue, supports Prometheus label templating"
    ///   expr: "rate(my_service_error_rate_metric[5m]) > 123"
    ///   min_duration: 15m
    ///   severity: warning
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prometheusAlerts: Vec<PrometheusAlert>,
}

impl Manifest {
    /// Set the version field
    pub fn version(mut self, version: String) -> Self {
        self.version = Some(version);
        self
    }

    /// Print manifest to stdout
    pub fn print(&self) -> Result<()> {
        println!("{}", serde_yaml::to_string(self)?);
        Ok(())
    }

    /// Verify the region for this manifest is one of its declared ones
    ///
    /// Assumes the manifest has been populated with `implicits`
    pub fn verify_region(&self) -> Result<&Self> {
        assert!(self.region != ""); // needs to have been set by implicits!
        if !self.regions.contains(&self.region.to_string()) {
            bail!("Unsupported region {} for service {}", self.region, self.name);
        };
        Ok(self)
    }

    /// Verifies the "destinationRules" manifest entries if they are configured
    ///
    /// It is erroneous to define destination rules without configuring the corresponding region's
    /// destination rules host regular expression
    pub fn verify_destination_rules(&self, region: &Region) -> Result<()> {
        if let Some(ref _destinationRules) = &self.destinationRules {
            if let Some(ref destinationRuleHostRegex) = &region.destinationRuleHostRegex {
                for dr in _destinationRules {
                    dr.verify(destinationRuleHostRegex)?;
                }
            } else {
                bail!("Cannot use `destinationRules` in a region without a `destinationRuleHostRegex`")
            }
        }
        Ok(())
    }

    /// Verify assumptions about manifest
    ///
    /// Assumes the manifest has been populated with `implicits`
    pub fn verify(&self, conf: &Config, region: &Region) -> Result<()> {
        self.verify_region()?;
        // limit to 50 characters, alphanumeric, dashes for sanity.
        // 63 is kube dns limit (13 char suffix buffer)
        let re = Regex::new(r"^[0-9a-z\-]{1,50}$").unwrap();
        if !re.is_match(&self.name) {
            bail!("Please use a short, lower case service names with dashes");
        }
        if self.name.ends_with('-') || self.name.starts_with('-') {
            bail!("Please use dashes to separate words only");
        }

        self.verify_destination_rules(region)?;

        // TODO: remove?
        if let Some(ref dh) = self.dataHandling {
            dh.verify()?
        }

        if let Some(ref md) = self.metadata {
            md.verify(&conf.owners, &conf.allowedCustomMetadata)?;
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

        // TODO [DIP-499]: Separate gate/kong params + adjust the checks
        if let Some(g) = &self.gate {
            if self.kongApis.is_empty() {
                bail!("Can't have a `gate` configuration without a `kong` one");
            }
            if g.public != self.publiclyAccessible {
                bail!("[Migration plan] `publiclyAccessible` and `gate.public` must be equal");
            }
        }

        // run the `Verify` trait on all imported structs
        // mandatory structs first
        if let Some(ref r) = self.resources {
            r.verify()?;
        } else {
            bail!("Resources is mandatory");
        }

        // optional/vectorised entries
        for d in &self.dependencies {
            d.verify()?;
        }

        for ha in &self.hostAliases {
            ha.verify()?;
        }
        for tl in &self.tolerations {
            tl.verify()?;
        }
        for r in &self.rbac {
            r.verify()?;
        }
        for pv in &self.persistentVolumes {
            pv.verify()?;
        }
        if let Some(ref cmap) = self.configs {
            cmap.verify()?;
        }
        for k in self.labels.keys() {
            if !conf.allowedLabels.contains(k) {
                bail!("Service: {} using label {} not defined in config", self.name, k)
            }
        }
        for es in &self.eventStreams {
            es.verify()?;
        }
        if let Some(kr) = &self.kafkaResources {
            kr.verify()?;
        }
        for pa in &self.prometheusAlerts {
            pa.verify()?;
        }
        // misc minor properties
        if self.replicaCount.unwrap() == 0 {
            bail!("Need replicaCount to be at least 1");
        }
        if let Some(ref ru) = &self.rollingUpdate {
            ru.verify(self.replicaCount.unwrap())?;
        }

        self.env.verify()?;

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
        if self.health.is_none() && self.readinessProbe.is_none() {
            warn!("{} does not set a health check", self.name)
        }

        Ok(())
    }

    fn get_vault_path(&self, vc: &VaultConfig) -> String {
        // some services use keys from other services
        let (svc, reg) = if let Some(ref vopts) = self.vault {
            (vopts.name.clone(), vc.folder.clone())
        } else {
            (self.name.clone(), vc.folder.clone())
        };
        format!("{}/{}", reg, svc)
    }

    // Get EnvVars for all containers, workers etc. for this Manifest.
    pub fn get_env_vars(&mut self) -> Vec<&mut EnvVars> {
        let mut envs = Vec::new();
        envs.push(&mut self.env);
        for s in &mut self.sidecars {
            envs.push(&mut s.env);
        }
        for w in &mut self.workers {
            envs.push(&mut w.container.env);
        }
        for c in &mut self.cronJobs {
            envs.push(&mut c.container.env);
        }
        for i in &mut self.initContainers {
            envs.push(&mut i.env);
        }
        envs
    }

    /// Populate placeholder fields with secrets from vault
    ///
    /// This will use the HTTP api of Vault using the configuration parameters
    /// in the `Config`.
    pub async fn secrets(&mut self, client: &Vault, vc: &VaultConfig) -> Result<()> {
        let pth = self.get_vault_path(vc);
        debug!("Injecting secrets from vault {} ({:?})", pth, client.mode());

        let mut vault_secrets = BTreeSet::new();
        let mut template_secrets = BTreeMap::new();
        for e in &mut self.get_env_vars() {
            for k in e.vault_secrets() {
                vault_secrets.insert(k.to_string());
            }
            for (k, v) in e.template_secrets() {
                let original = template_secrets.insert(k.to_string(), v.to_string());
                if original.iter().any(|x| x == &v) {
                    bail!(
                        "Secret {} can not be used in multiple templates with different values",
                        k
                    );
                }
            }
        }

        let template_keys = template_secrets.keys().map(|x| x.to_string()).collect();
        if let Some(k) = vault_secrets.intersection(&template_keys).next() {
            bail!("Secret {} can not be both templated and fetched from vault", k);
        }

        // Lookup values for each secret in vault.
        for k in vault_secrets {
            let vkey = format!("{}/{}", pth, k);
            self.secrets.insert(k.to_string(), client.read(&vkey).await?);
        }

        self.secrets.append(&mut template_secrets);

        // do the same for secret secrets
        for (k, v) in &mut self.secretFiles {
            if v == "IN_VAULT" {
                let vkey = format!("{}/{}", pth, k);
                *v = client.read(&vkey).await?;
            }
            // sanity check; secretFiles are assumed base64 verify we can decode
            if base64::decode(v).is_err() {
                bail!("Secret {} is not base64 encoded", k);
            }
        }
        Ok(())
    }

    /// Get a list of raw secrets (without associated keys)
    ///
    /// Useful for obfuscation mechanisms so it knows what to obfuscate.
    pub fn get_secrets(&self) -> Vec<String> {
        let mut secrets = vec![];
        for s in self.secrets.values() {
            secrets.push(s.clone());
            secrets.push(base64::encode(s));
        }
        secrets
    }

    pub async fn verify_secrets_exist(&self, vc: &VaultConfig) -> Result<()> {
        use std::collections::HashSet;
        // what are we requesting
        // TODO: Use envvars directly
        let keys = self
            .env
            .plain
            .clone()
            .into_iter()
            .filter(|(_, v)| v == "IN_VAULT")
            .map(|(k, _)| k)
            .collect::<HashSet<_>>();
        let files = self
            .secretFiles
            .clone()
            .into_iter()
            .filter(|(_, v)| v == "IN_VAULT")
            .map(|(k, _)| k)
            .collect::<HashSet<_>>();
        let expected = keys.union(&files).cloned().collect::<HashSet<_>>();
        if expected.is_empty() {
            return Ok(()); // no point trying to cross reference
        }

        // what we have
        let v = Vault::regional(vc)?;
        let secpth = self.get_vault_path(vc);

        // list secrets; fail immediately if folder is empty
        let found = match v.list(&secpth).await {
            Ok(lst) => lst.into_iter().collect::<HashSet<_>>(),
            Err(e) => bail!(
                "Missing secret folder {} expected to contain {:?}: {}",
                secpth,
                expected,
                e
            ),
        };
        debug!("Found secrets {:?} for {}", found, self.name);

        // compare sets
        let missing = expected.difference(&found).collect::<Vec<_>>();
        if !missing.is_empty() {
            bail!("Missing secrets: {:?} not found in vault {}", missing, secpth);
        }
        Ok(())
    }
}

// Cross-crate test manifest creator
impl Manifest {
    pub fn test(name: &str) -> Manifest {
        use serde_json::json;
        let mut mf: Manifest = serde_json::from_value(json!({
            "name": name,
            "version": "1.0.0",
            "regions": ["dev-uk"],
            // and it has defaults from filebacked:
            "chart": "base",
            // plus some mandatory properties normally not needed in tests
            "metadata": {
                "team": "doves",
                "repo": "https://github.com/babylonhealth/shipcat"
            },
        }))
        .expect("minimal manifest format is parseable");
        // fill some defaults normally done when loading it
        mf.namespace = "apps".into();
        mf.region = "dev-uk".into();
        mf.environment = "dev".into();
        mf
    }
}
