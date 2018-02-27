#![allow(non_snake_case)]

/// Allow normal error handling from structs
pub use super::Result;

// Structs that exist in the manifest

mod dependency;
pub use self::dependency::Dependency;

mod image;
pub use self::image::Image;

mod jaeger;
pub use self::jaeger::Jaeger;

// Kubernetes
/// Kube abstractions (not straight translations)
pub mod kube;

/// Kubernetes resource structs
pub mod resources;
/// Kubernetes volumes
pub mod volume;
/// Kubernetes host aliases
pub mod hostalias;
/// Kubernetes init containers
pub mod initcontainer;

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
