use walkdir::WalkDir;

use std::io::prelude::*;
use std::path::{PathBuf, Path};
use std::fs::File;
use super::vault::Vault;
use super::{Result, Manifest, Config};

/// Manifests backed by a manifests directory traverse the filesystem for discovery
impl Manifest {

    /// Walk the services directory and return the available services
    pub fn available() -> Result<Vec<String>> {
        let svcsdir = Path::new(".").join("services");
        let svcs = WalkDir::new(&svcsdir)
            .min_depth(1)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir());

        let mut xs = vec![];
        for e in svcs {
            let mut cmps = e.path().components();
            cmps.next(); // .
            cmps.next(); // services
            let svccomp = cmps.next().unwrap();
            let svcname = svccomp.as_os_str().to_str().unwrap();
            xs.push(svcname.into());
        }
        xs.sort();
        Ok(xs)
    }

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
    /// TODO: make private
    pub fn fill(&mut self, conf: &Config, region: &str) -> Result<()> {
        self.pre_merge_implicits(conf, Some(region.into()))?;
        // merge service specific env overrides if they exists
        let envlocals = Path::new(".")
            .join("services")
            .join(&self.name)
            .join(format!("{}.yml", region));
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
        self.post_merge_implicits(conf, Some(region.into()))?;
        Ok(())
    }

    /// Complete (filled in env overrides and populate secrets) a manifest
    pub fn completed(service: &str, conf: &Config, region: &str) -> Result<Manifest> {
        let r = &conf.regions[region]; // tested for existence earlier
        let v = Vault::regional(&r.vault)?;
        let pth = Path::new(".").join("services").join(service);
        if !pth.exists() {
            bail!("Service folder {} does not exist", pth.display())
        }
        let mut mf = Manifest::read_from(&pth)?;
        if !mf.regions.contains(&region.to_string()) {
            bail!("Service {} does not exist in the region {}", service, region);
        }
        // fill defaults and merge regions before extracting secrets
        mf.fill(conf, region)?;
        // replace one-off templates in evar strings with values
        mf.template_evars(conf, region)?;
        // secrets before configs (.j2 template files use raw secret values)
        mf.secrets(&v, &r.vault)?;
        // templates last
        mf.inline_configs(&conf, region)?;
        Ok(mf)
    }

    /// Mostly completed but stubbed secrets version of the manifest
    pub fn stubbed(service: &str, conf: &Config, region: &str) -> Result<Manifest> {
        let pth = Path::new(".").join("services").join(service);
        if !pth.exists() {
            bail!("Service folder {} does not exist", pth.display())
        }
        let mut mf = Manifest::read_from(&pth)?;
        mf.fill(conf, &region)?;
        Ok(mf)
    }

    /// Completed manifest with mocked values
    pub fn mocked(service: &str, conf: &Config, region: &str) -> Result<Manifest> {
        let r = &conf.regions[region]; // tested for existence earlier
        let v = Vault::mocked(&r.vault)?;
        let pth = Path::new(".").join("services").join(service);
        if !pth.exists() {
            bail!("Service folder {} does not exist", pth.display())
        }
        let mut mf = Manifest::read_from(&pth)?;
        // fill defaults and merge regions before extracting secrets
        mf.fill(conf, region)?;
        // replace one-off templates in evar strings with values
        mf.template_evars(conf, region)?;
        // (MOCKED) secrets before configs (.j2 template files use raw secret values)
        mf.secrets(&v, &r.vault)?;
        // templates last
        mf.inline_configs(&conf, region)?;
        Ok(mf)
    }

    /// A super base manifest - from an unknown region
    pub fn basic(service: &str, conf: &Config, region: Option<String>) -> Result<Manifest> {
        let pth = Path::new(".").join("services").join(service);
        if !pth.exists() {
            bail!("Service folder {} does not exist", pth.display())
        }
        let mut mf = Manifest::read_from(&pth)?;
        if mf.name != service {
            bail!("Service name must equal the folder name");
        }
        mf.pre_merge_implicits(conf, None)?;
        // not merging here, but do all implicts we can anyway
        mf.post_merge_implicits(conf, region)?;
        Ok(mf)
    }

}
