#![allow(non_snake_case)]

/// Allow normal error handling from structs
pub use super::{Result, ResultExt, ErrorKind, Error};

pub use super::config::{Config, VaultConfig, VersionScheme};

/// Manifests needs all the structs
pub use super::structs;

/// Needs vault client for secrets
pub use super::vault;

/// Main module
pub mod manifest;

// Re-exports
pub use self::manifest::Manifest;
pub use self::manifest::show;

// private module to define merge behaviour
mod merge;

/// A renderer of `tera` templates (jinja style)
///
/// Used for small app configs that are inlined in the completed manifests.
pub mod template;
