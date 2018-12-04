/// Allow normal error handling from structs
pub use super::{Result, ResultExt, ErrorKind, Error};
/// Verify trait gets the Config
pub use super::{Config, Region, VersionScheme, AuditWebhook};
/// Need basic manifest handling
pub use super::Manifest;

/// For slack hookback
pub use super::structs::Metadata;

// allow using some slack and kube stuff
pub use super::slack;
pub use super::grafana;
pub use super::kube;
pub use super::audit;

/// Parallel helm invokers
pub mod parallel;

/// Direct helm invokers (used by abstractions)
pub mod direct;
// Re-exports for main
pub use self::direct::{history, template, values, status};

/// Helm related helpers
pub mod helpers;
// Commonly used helper
pub use self::helpers::infer_fallback_version;

pub use self::direct::{UpgradeMode, UpgradeState, UpgradeData};
