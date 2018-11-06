use config::{Config, Region, ManifestDefaults};
use walkdir::WalkDir;

use std::io::prelude::*;
use std::path::{PathBuf, Path};
use std::fs::File;
use super::vault::Vault;
use super::{Result, ResultExt, ErrorKind, Manifest, ManifestType};
use traits::Backend;

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
    fn fill(&mut self, defaults: &ManifestDefaults, region: &Region) -> Result<()> {
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
        self.add_config_defaults(&defaults)?;
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

/// Manifests backed by a manifests directory traverse the filesystem for discovery
impl Backend for Manifest {
    fn available(region: &str) -> Result<Vec<String>> {
        let mut xs = vec![];
        for svc in walk_services() {
            let mf = Manifest::blank(&svc)?;
            if mf.regions.contains(&region.to_string()) && !mf.disabled && !mf.external {
                xs.push(svc);
            }
        }
        Ok(xs)
    }

    fn all() -> Result<Vec<String>> {
        Ok(walk_services())
    }


    /// A super base manifest - from an unknown region
    ///
    /// Can be used to read global Manifest values onlys
    fn blank(service: &str) -> Result<Manifest> {
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
    fn simple(service: &str, conf: &Config, reg: &Region) -> Result<Manifest> {
        let mut mf = Manifest::blank(service)?;
        // fill defaults and merge regions before extracting secrets
        mf.fill(&conf.defaults, reg)?;
        mf.kind = ManifestType::Simple;
        Ok(mf)
    }


    /// Create a CRD equivalent manifest
    fn base(service: &str, conf: &Config, reg: &Region) -> Result<Manifest> {
        let mut mf = Manifest::blank(service)?;
        // fill defaults and merge regions before extracting secrets
        mf.fill(&conf.defaults, reg)?;
        mf.read_configs_files()?;
        mf.kind = ManifestType::Base;

        Ok(mf)
    }

    /// Create a completed manifest with stubbed secrets (faster to retrieve)
    fn stubbed(service: &str, conf: &Config, reg: &Region) -> Result<Manifest> {
        let mut mf = Manifest::base(service, &conf, reg)?;
        mf.upgrade(reg, ManifestType::Stubbed)?;
        Ok(mf)
    }

    /// Create a completed manifest fetching secrets from Vault
    fn completed(service: &str, conf: &Config, reg: &Region) -> Result<Manifest> {
        let mut mf = Manifest::base(service, &conf, reg)?;
        mf.upgrade(reg, ManifestType::Completed)?;
        Ok(mf)
    }

    /// Upgrade a `Base` manifest to either a Complete or a Stubbed one
    fn upgrade(&mut self, reg: &Region, kind: ManifestType) -> Result<()> {
        assert_eq!(self.kind, ManifestType::Base); // sanity
        let v = match kind {
            ManifestType::Completed => Vault::regional(&reg.vault)?,
            ManifestType::Stubbed => Vault::mocked(&reg.vault)?,
            _ => bail!("Can only upgrade a Base manifest to Completed or Stubbed"),
        };
        // replace one-off templates in evar strings with values
        // note that this happens before secrets because:
        // secrets may be injected at this step from the Region
        self.template_evars(reg)?;
        // secrets before configs (.j2 template files use raw secret values)
        self.secrets(&v, &reg.vault)?;

        // templates last
        self.template_configs(reg)?;
        self.kind = kind;
        Ok(())
    }
}
