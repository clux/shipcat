#![allow(non_snake_case)]

/// Allow normal error handling from structs
pub use super::Result;
/// Verify trait gets the Region and Team
pub use super::{Region, Team};
/// Verify traits sometimes need to cross reference stuff from other manifests
pub use super::Manifest;

// Structs that exist in the manifest

mod dependency;
pub use self::dependency::{Dependency, DependencyProtocol};

mod worker;
pub use self::worker::Worker;

/// Kong configs
pub mod kong;
pub use self::kong::{Kong, Cors, BabylonAuthHeader, Authentication};

/// Gate configs
pub mod gate;
pub use self::gate::{Gate};

/// Kongfig configs
pub mod kongfig;
pub use self::kongfig::{Api, Consumer, Plugin, Upstream, Certificate};

/// Kafka configs
pub mod kafka;
pub use self::kafka::Kafka;

// Kubernetes - first are abstractions latter ones are straight translations

// abstractions - these have special handling
/// Templated configmap abstractions
mod configmap;
pub use self::configmap::{ConfigMap, ConfigMappedFile};
/// Healthcheck abstraction
mod healthcheck;
pub use self::healthcheck::HealthCheck;

mod env;
pub use self::env::EnvVars;

// translations - these are typically inlined in templates as yaml
/// Kubernetes resource structs
mod resources;
pub use self::resources::Resources;
pub use self::resources::parse_memory;
/// Kubernetes volumes
pub mod volume;
pub use self::volume::{Volume, VolumeMount};
/// Kubernetes host aliases
mod hostalias;
pub use self::hostalias::HostAlias;
/// Kubernetes init containers
mod initcontainer;
pub use self::initcontainer::InitContainer;
/// Kubernetes health check probes
mod probes;
pub use self::probes::Probe;
/// Kubernetes rolling-update settings
pub mod rollingupdate;
pub use self::rollingupdate::RollingUpdate;
/// Kubernetes horizontal pod autoscaler
pub mod autoscaling;
/// Kuberneter tolerations
pub mod tolerations;
/// Kubernetes container lifecycle events
mod lifecycle;
pub use self::lifecycle::{LifeCycle, LifeCycleHandler};

mod metadata;
pub use self::metadata::{Metadata, Contact, SlackChannel};


/// Security related structs
pub mod security;

mod vault;
pub use self::vault::VaultOpts;

/// Cron Jobs
pub mod cronjob;
pub use self::cronjob::CronJob;

/// Sidecar
pub mod sidecar;
pub use self::sidecar::Sidecar;

pub mod port;
pub use self::port::Port;

/// Rbac
pub mod rbac;
pub use self::rbac::Rbac;

// PersistentVolume
mod persistentvolume;
pub use self::persistentvolume::PersistentVolume;

/// AWS RDS
pub mod rds;
/// AWS ElastiCache
pub mod elasticache;
