/// Allow normal error handling from structs
pub use super::{Result, ErrorKind};
/// Verify trait gets the Config
pub use super::config::{Config, RegionDefaults};

// allow using some slack and templating stuff
pub use super::template;
pub use super::slack;
pub use super::kube;
pub use super::generate;
pub use super::Manifest;
pub use super::vault;

/// Parallel helm invokers
pub mod parallel;

/// Direct helm invokers (used by abstractions)
pub mod direct;
// Re-exports for main
pub use self::direct::{history, template, diff, values};
// Re-export that should be wrapped in parallel dependency inferrer later on
pub use self::direct::upgrade;

/// Helm related helpers
pub mod helpers;
// Commonly used helper
pub use self::helpers::infer_fallback_version;

pub use self::direct::{UpgradeMode};
