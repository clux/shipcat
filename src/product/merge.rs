// This file describes how product manifests and environment overrides are merged.

use serde_yaml;
use std::path::PathBuf;
use std::io::prelude::*;
use std::fs::File;

use super::{Product, Result, Config};

impl Product {

    /// Add implicit defaults to self after merging in location overrides
    pub fn post_merge_implicits(&mut self, _conf: &Config, location: Option<String>) -> Result<()> {
        if let Some(l) = location {
            self.location = l.clone();
        }
        Ok(())
    }
    /// Add implicit defaults to self before merging in values
    pub fn pre_merge_implicits(&mut self, _conf: &Config) -> Result<()> {
        // currently nothing like this required
        Ok(())
    }


    /// Merge defaults from partial override file
    ///
    /// Copies keys from environment files into the current product struct by default.
    pub fn merge(&mut self, pth: &PathBuf) -> Result<()> {
        trace!("Merging {}", pth.display());
        if !pth.exists() {
            bail!("Defaults file {} does not exist", pth.display())
        }
        let mut f = File::open(&pth)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        // Because Product has most things implementing Default via serde
        // we can put this straight into a Product struct
        let px: Product = serde_yaml::from_str(&data)?;

        // sanity asserts
        if !px.locations.is_empty() {
            // these cannot be overridden - it's a service type property
            bail!("Locations must only be defined in the main product.yml file");
        }
        if self.version.is_some() {
            warn!("{} locks versions across all locations in product.yml", self.name);
        }

        // optional values that are replaced if present in override
        if px.version.is_some() {
            self.version = px.version;
        }
        if px.jira.is_some() {
            self.jira = px.jira;
        }
        if px.owner.is_some() {
            self.owner = px.owner;
        }

        // vectors are replaced if they are non-empty in override
        if !px.services.is_empty() {
            self.services = px.services;
        }

        Ok(())
    }

}
