/// Allow normal error handling from structs
pub use super::{Result, ErrorKind, Error};
/// Verify trait gets the Config
pub use super::config::{Config, VersionScheme};
/// For slack hookback
pub use super::structs::Metadata;

// allow using some slack and kube stuff
pub use super::slack;
pub use super::kube;
pub use super::Manifest;

/// Parallel helm invokers
pub mod parallel;

/// Direct helm invokers (used by abstractions)
pub mod direct;
// Re-exports for main
pub use self::direct::{history, template, values};

/// Helm related helpers
pub mod helpers;
// Commonly used helper
pub use self::helpers::infer_fallback_version;

pub use self::direct::{UpgradeMode, UpgradeData};
