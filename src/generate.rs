use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::prelude::*;
use std::io;

use serde_yaml;

use tera::Context; // just a hashmap wrapper
use super::structs::ConfigMappedFile;
use super::{Result};
use super::manifest::*;

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


// base context with variables used by templates
fn make_base_context(dep: &Deployment) -> Result<Context> {
    let mut ctx = Context::new();
    ctx.add("namespace", &dep.manifest.namespace);
    ctx.add("env", &dep.manifest.env);
    ctx.add("service", &dep.service);
    ctx.add("region", &dep.region);
    Ok(ctx)
}


fn template_config(dep: &Deployment, mount: &ConfigMappedFile) -> Result<String> {
    let ctx = make_base_context(dep)?;
    Ok((dep.render)(&mount.name, &ctx)?)
}

/// Helper to create a local OUTPUT directory
pub fn create_output(pwd: &PathBuf) -> Result<()> {
    let loc = pwd.join("OUTPUT");
    if loc.is_dir() {
        fs::remove_dir_all(&loc)?;
    }
    fs::create_dir(&loc)?;
    Ok(())
}

/// Deployment parameters and context bound helpers
pub struct Deployment {
    /// Service name (same as manifest.name)
    pub service: String,
    /// Region parameter
    pub region: String,
    /// Manifest
    pub manifest: Manifest,
    /// Optional semver version
    pub version: Option<String>,
    /// Context bound template render function
    pub render: Box<Fn(&str, &Context) -> Result<(String)>>,
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
pub fn helm(dep: &Deployment, output: Option<String>) -> Result<Manifest> {
    dep.check()?; // sanity check on deployment
    let mut mf = dep.manifest.clone();

    // Files in `ConfigMap` get pre-rendered for helm for now
    if let Some(ref mut cfg) = mf.configs {
        for f in &mut cfg.files {
            let res = template_config(dep, &f)?;
            f.value = Some(res);
        }
    }
    // pass overridden version into manifest
    if let Some(v) = dep.version.clone() {
        mf.version = Some(v);
    }

    let encoded = serde_yaml::to_string(&mf)?;
    if let Some(o) = output {
        let pth = Path::new(".").join(o);
        info!("Writing helm values for {} to {}", dep.service, pth.display());
        let mut f = File::create(&pth)?;
        write!(f, "{}\n", encoded)?;
        debug!("Wrote helm values for {} to {}: \n{}", dep.service, pth.display(), encoded);
    } else {
        // stdout only
        print!("{}", encoded);
        io::stdout().flush()?; // allow piping stdout elsewhere
    }
    Ok(mf)
}

#[cfg(test)]
mod tests {
    use super::{helm, Deployment};
    use super::super::Manifest;
    use super::super::template;
    use tests::setup;

    #[test]
    fn helm_create() {
        setup();
        let tera = template::init("fake-ask".into()).unwrap();
        let dep = Deployment {
            service: "fake-ask".into(),
            region: "dev-uk".into(),
            version: None,
            manifest: Manifest::basic("fake-ask").unwrap(),
            // only provide template::render as the interface (move tera into this)
            render: Box::new(move |tmpl, context| {
                template::render(&tera, tmpl, context)
            }),
        };
        if let Err(e) = helm(&dep, None) {
            println!("Failed to create helm values for fake-ask");
            print!("{}", e);
            assert!(false);
        }
        // can verify output here matches what we want if we wanted to,
        // but type safety proves 99% of that anyway
    }
}
