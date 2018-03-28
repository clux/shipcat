use std::path::Path;
use std::fs::File;
use std::io::prelude::*;
use std::io;

use serde_yaml;

use tera::{Context, Tera};
use super::structs::ConfigMappedFile;
use super::{Result};
use super::manifest::*;
use super::template;

/// Rendered `ConfigMap`
#[derive(Serialize, Clone, Default)]
pub struct ConfigMapRendered {
    pub name: String,
    pub path: String,
    pub files: Vec<RenderedConfig>,
}
/// Rendered `ConfigMappedFile`
#[derive(Serialize, Clone, Default)]
pub struct RenderedConfig {
    pub name: String,
    pub rendered: String,
}


fn template_config(dep: &Deployment, tera: &Tera, mount: &ConfigMappedFile) -> Result<String> {
    let mut ctx = Context::new();
    ctx.add("env", &dep.manifest.env);
    ctx.add("service", &dep.service);
    ctx.add("region", &dep.region);
    template::render(tera, &mount.name, &ctx)
}

/// Deployment parameters and context bound helpers
pub struct Deployment {
    /// Service name (same as manifest.name)
    pub service: String,
    /// Region parameter
    pub region: String,
    /// Manifest
    pub manifest: Manifest,
}
impl Deployment {
    pub fn check(&self) -> Result<()> {
        if self.service != self.manifest.name {
            bail!("manifest name does not match service name");
        }
        if !self.manifest.regions.contains(&self.region) {
            warn!("Using region '{}', but supported regions: {:?}", self.region, self.manifest.regions);
            bail!("manifest does not contain specified region");
        }
        Ok(())
    }
}

/// Helm values writer
///
/// Fills in service specific config files into config to help helm out
pub fn helm(dep: &Deployment, output: Option<String>, tera: &Tera, silent: bool) -> Result<Manifest> {
    dep.check()?; // sanity check on deployment
    let mut mf = dep.manifest.clone();

    // Files in `ConfigMap` get pre-rendered for helm for now
    if let Some(ref mut cfg) = mf.configs {
        for f in &mut cfg.files {
            let res = template_config(dep, tera, &f)?;
            f.value = Some(res);
        }
    }

    let encoded = serde_yaml::to_string(&mf)?;
    if let Some(o) = output {
        let pth = Path::new(".").join(o);
        if silent {
            debug!("Writing helm values for {} to {}", dep.service, pth.display());
        } else {
            info!("Writing helm values for {} to {}", dep.service, pth.display());
        }
        let mut f = File::create(&pth)?;
        write!(f, "{}\n", encoded)?;
        debug!("Wrote helm values for {} to {}: \n{}", dep.service, pth.display(), encoded);
    } else {
        // stdout only
        let _ = io::stdout().write(format!("{}\n", encoded).as_bytes());
    }
    Ok(mf)
}

#[cfg(test)]
mod tests {
    use super::{helm, Deployment};
    use super::super::Manifest;
    use super::super::template;
    use tests::setup;
    use super::super::Config;

    #[test]
    fn helm_create() {
        setup();
        let tera = template::init("fake-ask".into()).unwrap();
        let conf = Config::read().unwrap();
        let dep = Deployment {
            service: "fake-ask".into(),
            region: "dev-uk".into(),
            manifest: Manifest::basic("fake-ask", &conf, Some("dev-uk".into())).unwrap(),
        };
        if let Err(e) = helm(&dep, None, &tera, false) {
            println!("Failed to create helm values for fake-ask");
            print!("{}", e);
            assert!(false);
        }
        // can verify output here matches what we want if we wanted to,
        // but type safety proves 99% of that anyway
    }
}
