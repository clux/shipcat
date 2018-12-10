use std::io::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::{Config, Region, Manifest};
use super::Result;
use crate::states::{ManifestType};

/// Private helpers for a filebacked Manifest Backend
impl Manifest {
    /// Read a manifest file in an arbitrary path
    fn read_from(pwd: &PathBuf) -> Result<Manifest> {
        let mpath = pwd.join("shipcat.yml");
        trace!("Using manifest in {}", mpath.display());
        if !mpath.exists() {
            bail!("Manifest file {} does not exist", mpath.display())
        }
        let mut f = File::open(&mpath)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        Ok(serde_yaml::from_str(&data)?)
    }


    /// Fill in env overrides and apply merge rules
    fn merge_and_fill_defaults(&mut self, conf: &Config, region: &Region) -> Result<()> {
        // merge service specific env overrides if they exists
        let envlocals = Path::new(".")
            .join("services")
            .join(&self.name)
            .join(format!("{}.yml", region.name));
        if envlocals.is_file() {
            debug!("Merging environment locals from {}", envlocals.display());
            if !envlocals.exists() {
                bail!("Defaults file {} does not exist", envlocals.display())
            }
            let mut f = File::open(&envlocals)?;
            let mut data = String::new();
            f.read_to_string(&mut data)?;
            if data.is_empty() {
                bail!("Environment override file {} is empty", envlocals.display());
            }
            // Because Manifest has most things implementing Default via serde
            // we can put this straight into a Manifest struct
            let other: Manifest = serde_yaml::from_str(&data)?;

            self.merge(other)?;
        }
        self.add_config_defaults(&conf)?;
        self.add_region_implicits(region)?;
        Ok(())
    }
}

fn walk_services() -> Vec<String> {
    let svcsdir = Path::new(".").join("services");
    let mut res : Vec<_> = WalkDir::new(&svcsdir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
        .map(|e| {
            let mut cmps = e.path().components();
            cmps.next(); // .
            cmps.next(); // services
            let svccomp = cmps.next().unwrap();
            let svcname = svccomp.as_os_str().to_str().unwrap();
            svcname.to_string()
        })
        .collect();
    res.sort();
    res
}


/// Filesystem accessors for Manifest
impl Manifest {
    pub fn available(region: &str) -> Result<Vec<String>> {
        let mut xs = vec![];
        for svc in walk_services() {
            let mf = Manifest::blank(&svc)?;
            if mf.regions.contains(&region.to_string()) && !mf.disabled && !mf.external {
                xs.push(svc);
            }
        }
        Ok(xs)
    }

    /// Create an-all pieces manifest ready to be upgraded
    ///
    /// The CRD equivalent that has templates read from disk first.
    pub fn base(service: &str, conf: &Config, reg: &Region) -> Result<Manifest> {
        let mut mf = Manifest::blank(service)?;
        // fill defaults and merge regions before extracting secrets
        mf.merge_and_fill_defaults(&conf, reg)?;
        mf.read_configs_files()?;
        mf.kind = ManifestType::Base;

        Ok(mf)
    }

    /// Return all services found in the manifests services folder
    pub fn all() -> Result<Vec<String>> {
        Ok(walk_services())
    }

    /// A super base manifest - from an unknown region
    ///
    /// Can be used to read global Manifest values onlys
    pub fn blank(service: &str) -> Result<Manifest> {
        let pth = Path::new(".").join("services").join(service);
        if !pth.exists() {
            bail!("Service folder {} does not exist", pth.display())
        }
        let mf = Manifest::read_from(&pth)?;
        if mf.name != service {
            bail!("Service name must equal the folder name");
        }
        Ok(mf)
    }

    /// Create a simple manifest that has enough for most reducers
    pub fn simple(service: &str, conf: &Config, reg: &Region) -> Result<Manifest> {
        let mut mf = Manifest::blank(service)?;
        // fill defaults and merge regions before extracting secrets
        mf.merge_and_fill_defaults(&conf, reg)?;
        mf.kind = ManifestType::Simple;
        Ok(mf)
    }
}
