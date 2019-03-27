use std::fmt;

use super::structs::{Metadata};

/// Subset of a service manifest without any region-level defaults/overrides.
#[derive(Clone)]
pub struct BaseManifest {
    /// Name of service
    pub name: String,
    pub metadata: Metadata,
    pub regions: Vec<String>,
}

impl fmt::Debug for BaseManifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BaseManifest {{ name: {} }}", self.name)
    }
}
