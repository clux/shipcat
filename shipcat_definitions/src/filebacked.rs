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
        self.add_struct_implicits()?;
        self.add_region_implicits(region)?;
        Ok(())
    }

    /// A super base manifest - from an unknown region
    ///
    /// Can be used to read global Manifest values only.
    fn blank(service: &str) -> Result<Manifest> {
        let pth = Path::new(".").join("services").join(service);
        if !pth.exists() {
            bail!("Service folder {} does not exist", pth.display())
        }
        let mf = Manifest::read_from(&pth)?;
        if mf.name != service {
            bail!("Service name must equal the folder name");
        }
        // maybe do this if we add these to sig:
        // defs: &ManifestDefaults, region: Option<&Region>
        // no merging, but can add defaults + implicits from conffig anyway
        //self.add_config_defaults(&defaults)?;
        //self.add_struct_implicits()?;
        //self.add_region_implicits(region)?; (if option is Some)
        Ok(mf)
    }
}


/// Manifests backed by a manifests directory traverse the filesystem for discovery
impl Backend for Manifest {
    fn available(region: &str) -> Result<Vec<String>> {
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
            let mf = Manifest::blank(svcname)?;
            if mf.regions.contains(&region.to_string()) && !mf.disabled && !mf.external {
                xs.push(svcname.into());
            }
        }
        xs.sort();
        Ok(xs)
    }

    fn completed(service: &str, conf: &Config, reg: &Region) -> Result<Manifest> {
        let mut mf = Manifest::base(service, &conf, reg)?;
        mf.upgrade(reg, ManifestType::Completed)?;
        Ok(mf)
    }

    fn base(service: &str, conf: &Config, reg: &Region) -> Result<Manifest> {
        let pth = Path::new(".").join("services").join(service);
        if !pth.exists() {
            bail!("Service folder {} does not exist", pth.display())
        }
        let mut mf = Manifest::read_from(&pth)?;
        if !mf.regions.contains(&reg.name) {
            bail!("Service {} does not exist in the region {}", service, reg.name);
        }
        // fill defaults and merge regions before extracting secrets
        mf.fill(&conf.defaults, reg)?;
        mf.read_configs_files()?;
        mf.kind = ManifestType::Base;

        Ok(mf)
    }

    fn stubbed(service: &str, conf: &Config, reg: &Region) -> Result<Manifest> {
        let mut mf = Manifest::base(service, &conf, reg)?;
        mf.upgrade(reg, ManifestType::Stubbed)?;
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

    /// Verifying a manifest in the filesystem
    ///
    /// Needs to account for the manifest being external/disabled, then secrets.
    fn validate(svc: &str, conf: &Config, region: &Region, secrets: bool) -> Result<()> {
        let bm = Manifest::blank(svc)?;

        if bm.regions.contains(&region.name) && !bm.disabled && !bm.external {
            let mf = if secrets {
                Manifest::completed(&svc, &conf, &region)?
            } else {
                Manifest::stubbed(&svc, &conf, &region)?
            };
            mf.verify(conf, region).chain_err(|| ErrorKind::InvalidManifest(mf.name))?;
        } else {
            bail!("{} is not configured to be deployed in {}", svc, region.name);
        }
        Ok(())
    }
}
