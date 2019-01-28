// This file describes how manifests and environment manifest overrides are merged.

use super::{Config, Region};
use super::{Manifest, Result};

impl Manifest {
    /// Add implicit defaults to self after merging in region overrides
    ///
    /// Should be used by entries that have complex implicit results that can be partially overridden
    /// I.e. kong struct, dataHandling structs which both have implicit values
    pub fn add_region_implicits(&mut self, reg: &Region) -> Result<()> {
        // environment defaults for a region is merged in only if not set explictly
        for (k, v) in reg.env.clone() {
            self.env.plain.entry(k).or_insert(v);
        }

        // Kong has implicit, region-scoped values
        if let Some(ref mut kong) = self.kong {
            kong.implicits(self.name.clone(), reg.clone(), self.hosts.clone());
        }
        if let Some(ref mut kafka) = self.kafka {
            kafka.implicits(&self.name, reg.clone())
        }
        if let Some(ref mut dh) = self.dataHandling {
            // dataHandling has cascading encryption values
            dh.implicits();
        }
        // infrastructure uses service name as database name
        // it also passes on the team from metadata:
        let md = self.metadata.clone().unwrap(); // exists by merge_and_fill_defaults
        if let Some(ref mut db) = self.database {
            db.implicits(&self.name, &md);
        }
        if let Some(ref mut redis) = self.redis {
            redis.implicits(&self.name, &md);
        }

        // Inject the region's environment name and namespace
        self.environment = reg.environment.clone();
        self.namespace = reg.namespace.clone();
        self.region = reg.name.clone();

        Ok(())
    }

    /// Add implicit defaults to from the config
    ///
    /// Should be used by entries that have simple implicit results based on the config
    /// I.e. optional strings, integers etc.
    pub fn add_config_defaults(&mut self, conf: &Config) -> Result<()> {
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
            self.replicaCount = Some(conf.defaults.replicaCount);
        }
        if let Some(ref mut md) = &mut self.metadata {
            let team = if let Some(t) = conf.teams.iter().find(|t| t.name == md.team) {
                t
            } else {
                bail!("The team name must match one of the team names in shipcat.conf");
            };
            if md.support.is_none() {
                md.support = team.support.clone();
            }
            if md.notifications.is_none() {
                md.notifications = team.notifications.clone();
            }
        }

        Ok(())
    }


    /// Merge defaults from partial override file
    ///
    /// Copies keys from environment files into the current manifest struct by default.
    /// One special cases are merged carefully:
    /// - env dict (merged by key)
    pub fn merge(&mut self, mf: Manifest) -> Result<()> {
        if mf.name != "" {
            bail!("Cannot override service names in other environments");
        }
        // sanity asserts
        if self.kong.is_some() && mf.kong.is_some() {
            // Must override Kong per environment (overwrite full struct)
            bail!("Cannot have kong in main shipcat.yml and environment override files");
        }
        if !mf.regions.is_empty() {
            // these cannot be overridden - it's a service type property
            bail!("Regions must only be defined in the main shipcat.yml file");
        }
        if mf.metadata.is_some() {
            bail!("metadata can only live in the main shipcat.yml")
        }
        //if self.version.is_some() {
        //    debug!("{} locks versions across all environments in shipcat.yml", self.name);
        //}

        // start merging:

        // merge maps by appending to keys found in shipcat.yml
        for (k, v) in mf.env.plain {
            self.env.plain.insert(k, v);
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
        if mf.kafka.is_some() {
            self.kafka = mf.kafka;
        }
        if mf.kong.is_some() {
            self.kong = mf.kong;
        }
        if mf.database.is_some() {
            self.database = mf.database;
        }
        if mf.redis.is_some() {
            self.redis = mf.redis;
        }
        if mf.vault.is_some() {
            self.vault = mf.vault;
        }
        if mf.rollingUpdate.is_some() {
            self.rollingUpdate = mf.rollingUpdate;
        }
        if mf.autoScaling.is_some() {
            self.autoScaling = mf.autoScaling;
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
        if mf.externalPort.is_some() {
            self.externalPort = mf.externalPort;
        }
        if mf.lifecycle.is_some() {
            self.lifecycle = mf.lifecycle;
        }

        // vectors are replaced if they are non-empty in override
        if !mf.command.is_empty() {
            self.command = mf.command;
        }
        if !mf.hosts.is_empty() {
            self.hosts = mf.hosts;
        }
        if !mf.hostAliases.is_empty() {
            self.hostAliases = mf.hostAliases;
        }
        if !mf.tolerations.is_empty() {
            self.tolerations = mf.tolerations;
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
        if !mf.persistentVolumes.is_empty() {
            self.persistentVolumes = mf.persistentVolumes;
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
        if !mf.sourceRanges.is_empty() {
            self.sourceRanges = mf.sourceRanges;
        }

        if !mf.ports.is_empty() {
            self.ports = mf.ports;
        }

        Ok(())
    }
}
