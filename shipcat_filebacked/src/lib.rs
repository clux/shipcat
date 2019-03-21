#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate merge_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate error_chain;

// Structs
mod manifest;

// Utilities
mod load;

use manifest::ManifestSource;
use shipcat_definitions::{Config, Manifest, Region, SimpleManifest, Result};

pub fn load_manifest(service: &str, conf: &Config, reg: &Region) -> Result<Manifest> {
    ManifestSource::load_manifest(service, conf, reg)
}

pub fn load_metadata(service: &str, conf: &Config, reg: &Region) -> Result<SimpleManifest> {
    ManifestSource::load_metadata(service, conf, reg)
}
