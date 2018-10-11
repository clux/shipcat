#![allow(non_snake_case)]

/// Allow normal error handling from structs
pub use super::{Result, ResultExt, ErrorKind, Error};

pub use super::config::{Config, VaultConfig, VersionScheme};

/// Manifests needs all the structs
pub use super::structs;

/// Needs vault client for secrets
pub use super::vault;

/// Reducers used by shipcat get
pub mod reducers;

/// Main module
pub mod manifest;

/// File backing
pub mod filebacked;
// merge behaviour for file backed manifests
mod merge;

// TODO: CRD backing

/// Computational helpers
pub mod math;

// Re-exports
pub use self::manifest::Manifest;
pub use self::filebacked::show;



/// A renderer of `tera` templates (jinja style)
///
/// Used for small app configs that are inlined in the completed manifests.
pub mod template;
