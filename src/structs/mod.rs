#![allow(non_snake_case)]

// Structs that exist in the manifest

mod dependency;
pub use self::dependency::Dependency;

mod image;
pub use self::image::Image;

mod jaeger;
pub use self::jaeger::Jaeger;

/// Kubernetes structs
pub mod kube;

mod metadata;
pub use self::metadata::Metadata;

/// Prometheus structs
pub mod prometheus;

mod security;
pub use self::security::DataHandling;

mod vault;
pub use self::vault::VaultOpts;
