// This file describes how manifests and environment manifest overrides are merged.

use serde_yaml;
use std::path::PathBuf;
use std::io::prelude::*;
use std::fs::File;

use super::{Manifest, Result, Config};

impl Manifest {

    /// Add implicit defaults to self after merging in region overrides
    ///
    /// Should be used by entries that have complex implicit results that can be partially overridden
    /// I.e. kong struct, dataHandling structs which both have implicit values
    pub fn post_merge_implicits(&mut self, conf: &Config, region: Option<String>) -> Result<()> {
        if let Some(r) = region {
            self.region = r.clone();
            let reg = conf.get_region(&r)?;
            for (k, v) in reg.env {
                self.env.insert(k, v);
            }

            // Kong has implicit, region-scoped values
            if let Some(ref mut kong) = self.kong {
                kong.implicits(self.name.clone(), conf.regions[&r].clone());
            }

            // Inject the region environment
            self.environment = reg.defaults.environment;
        }
        if let Some(ref mut dh) = self.dataHandling {
            // dataHandling has cascading encryption values
            dh.implicits();
        }
        if let Some(ref mut cfg) = self.configs {
            cfg.implicits(&self.name)
        }
        for d in &mut self.dependencies {
            d.implicits();
        }
        Ok(())
    }

    /// Add implicit defaults to self before merging in values
    ///
    /// Should be used by entries that have simple implicit results based on the config
    /// I.e. optional strings, integers etc.
    pub fn pre_merge_implicits(&mut self, conf: &Config) -> Result<()> {
        if self.image.is_none() {
            // image name defaults to some prefixed version of the service name
            self.image = Some(format!("{}/{}", conf.defaults.imagePrefix, self.name))
        }
        if self.imageSize.is_none() {
            self.imageSize = Some(512)
        }
        if self.chart.is_none() {
            self.chart = Some(conf.defaults.chart.clone());
        }
        if self.replicaCount.is_none() {
            self.replicaCount = Some(conf.defaults.replicaCount)
        }

        Ok(())
    }


    /// Merge defaults from partial override file
    ///
    /// Copies keys from environment files into the current manifest struct by default.
    /// One special cases are merged carefully:
    /// - env dict (merged by key)
    pub fn merge(&mut self, pth: &PathBuf) -> Result<()> {
        trace!("Merging {}", pth.display());
        if !pth.exists() {
            bail!("Defaults file {} does not exist", pth.display())
        }
        let mut f = File::open(&pth)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        // Because Manifest has most things implementing Default via serde
        // we can put this straight into a Manifest struct
        let mf: Manifest = serde_yaml::from_str(&data)?;

        // sanity asserts
        if self.kong.is_some() && mf.kong.is_some() {
            // Must override Kong per environment (overwrite full struct)
            bail!("Cannot have kong in main shipcat.yml and environment override files");
        }
        if !mf.regions.is_empty() {
            // these cannot be overridden - it's a service type property
            bail!("Regions must only be defined in the main shipcat.yml file");
        }
        if self.version.is_some() {
            warn!("{} locks versions across all environments in shipcat.yml", self.name);
        }

        // start merging:

        // merge maps by appending to keys found in shipcat.yml
        for (k,v) in mf.env {
            self.env.insert(k, v);
        }
        for (k,v) in mf.secretFiles {
            self.secretFiles.insert(k, v);
        }
        for (k,v) in mf.serviceAnnotations {
            self.serviceAnnotations.insert(k, v);
        }

        // optional values that are replaced if present in override
        if mf.chart.is_some() {
            self.chart = mf.chart;
        }
        if mf.image.is_some() {
            self.image = mf.image;
        }
        if mf.imageSize.is_some() {
            self.imageSize = mf.imageSize;
        }
        if mf.version.is_some() {
            self.version = mf.version;
        }
        if mf.metadata.is_some() {
            self.metadata = mf.metadata;
        }
        if mf.dataHandling.is_some() {
            self.dataHandling = mf.dataHandling;
        }
        if mf.language.is_some() {
            self.language = mf.language;
        }
        if mf.replicaCount.is_some() {
            self.replicaCount = mf.replicaCount;
        }
        if mf.configs.is_some() {
            self.configs = mf.configs;
        }
        if mf.resources.is_some(){
            self.resources = mf.resources;
        }
        if mf.kong.is_some() {
            self.kong = mf.kong;
        }
        if mf.vault.is_some() {
            self.vault = mf.vault;
        }
        if mf.health.is_some() {
            self.health = mf.health;
        }
        if mf.readinessProbe.is_some() {
            self.readinessProbe = mf.readinessProbe;
        }
        if mf.livenessProbe.is_some() {
            self.livenessProbe = mf.livenessProbe;
        }
        if mf.httpPort.is_some() {
            self.httpPort = mf.httpPort;
        }

        // vectors are replaced if they are non-empty in override
        if !mf.command.is_empty() {
            self.command = mf.command;
        }
        if !mf.hostAliases.is_empty() {
            self.hostAliases = mf.hostAliases;
        }
        if !mf.volumeMounts.is_empty() {
            self.volumeMounts = mf.volumeMounts;
        }
        if !mf.initContainers.is_empty() {
            self.initContainers = mf.initContainers;
        }
        if !mf.volumes.is_empty() {
            self.volumes = mf.volumes;
        }
        if !mf.cronJobs.is_empty() {
            self.cronJobs = mf.cronJobs;
        }
        if !mf.dependencies.is_empty() {
            self.dependencies = mf.dependencies;
        }
        if !mf.sidecars.is_empty() {
            self.sidecars = mf.sidecars;
        }
        if !mf.workers.is_empty() {
            self.workers = mf.workers;
        }
        if !mf.labels.is_empty() {
            self.labels = mf.labels;
        }

        Ok(())
    }

}
