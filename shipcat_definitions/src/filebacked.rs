use std::io::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::{Manifest, Result};
use super::states::ManifestType;

/// Private helpers for a filebacked Manifest Backend
impl Manifest {
    /// Read a manifest file in an arbitrary path
    fn read_from(mpath: &PathBuf) -> Result<Self> {
        trace!("Reading manifest in {}", mpath.display());
        if !mpath.exists() {
            bail!("Manifest file {} does not exist", mpath.display())
        }
        let mut f = File::open(&mpath)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        if data.is_empty() {
            bail!("Manifest file {} is empty", mpath.display());
        }
        let mut mf: Manifest = serde_yaml::from_str(&data)?;
        mf.kind = ManifestType::SingleFile;
        Ok(mf)
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
        let mf = Manifest::read_from(&pth.join("shipcat.yml"))?;
        if mf.name != service {
            bail!("Service name must equal the folder name");
        }
        Ok(mf)
    }
}
