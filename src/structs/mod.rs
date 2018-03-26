#![allow(non_snake_case)]

/// Allow normal error handling from structs
pub use super::Result;

// Structs that exist in the manifest

mod dependency;
pub use self::dependency::{Dependency, DependencyProtocol};

mod jaeger;
pub use self::jaeger::Jaeger;

// Kubernetes - first are abstractions latter ones are straight translations

// abstractions - these have special handling
/// Templated configmap abstractions
mod config;
pub use self::config::{ConfigMap, ConfigMappedFile};
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


mod metadata;
pub use self::metadata::Metadata;

/// Prometheus structs
pub mod prometheus;


mod security;
pub use self::security::DataHandling;

mod vault;
pub use self::vault::VaultOpts;

/// Traits that the structs can implement
pub mod traits;

/// Cron Jobs
pub mod cronjob;
pub use self::cronjob::CronJob;

