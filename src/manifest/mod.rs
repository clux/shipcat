#![allow(non_snake_case)]

/// Allow normal error handling from structs
pub use super::{Result, ErrorKind, Error};

pub use super::config::{Config, RegionDefaults};

/// Manifests needs all the structs
pub use super::structs;

/// Needs vault client for secrets
pub use super::vault;

/// Parallel helm invokers
pub mod manifest;

// Exports
pub use self::manifest::Manifest;

// private module to define merge behaviour
mod merge;

/// A renderer of `tera` templates (jinja style)
///
/// Used for small app configs that are inlined in the completed manifests.
pub mod template;
