#![allow(non_snake_case)]

/// Allow normal error handling from structs
pub use super::Result;
/// Verify trait gets the Config
pub use super::Config;
/// Verify traits sometimes need to cross reference stuff from other manifests
pub use super::Manifest;

// Structs that exist in the manifest

mod dependency;
pub use self::dependency::{Dependency, DependencyProtocol};

mod worker;
pub use self::worker::Worker;

mod jaeger;
pub use self::jaeger::Jaeger;

/// Kong configs
pub mod kong;
pub use self::kong::Kong;

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


// translations - these are typically inlined in templates as yaml
/// Kubernetes resource structs
mod resources;
pub use self::resources::Resources;
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
mod rollingupdate;
pub use self::rollingupdate::RollingUpdate;
/// Kubernetes horizontal pod autoscaler
pub mod autoscaling;
/// Kuberneter tolerations
pub mod tolerations;

mod metadata;
pub use self::metadata::{Metadata, Contact};

/// Prometheus structs
pub mod prometheus;


/// Security related structs
pub mod security;

mod vault;
pub use self::vault::VaultOpts;

/// Traits that the structs can implement
pub mod traits;

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
