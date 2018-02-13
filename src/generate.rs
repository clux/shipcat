use tera::Context; // just a hashmap wrapper
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
    // env-loc == region
    let region = format!("{}-{}", dep.environment, dep.location);

    let mut ctx = Context::new();
    ctx.add("env", &dep.manifest.env);
    ctx.add("service", &dep.service);
    ctx.add("region", &region);
    Ok(ctx)
}

// full context modifier with all variables used by deployment templates as well
fn make_full_deployment_context(dep: &Deployment) -> Result<Context> {
    let mut ctx = make_base_context(dep)?;

    // Files in `ConfigMap` get pre-rendered with a sanitized template context
    if let Some(cfg) = dep.manifest.configs.clone() {
        let mut files = vec![];
        for f in cfg.files {
            let res = template_config(dep, &f)?;
            files.push(RenderedConfig { name: f.dest.clone(), rendered: res });
        }
        let config = ConfigMapRendered {
            name: cfg.name.unwrap(), // filled in by implicits
            path: cfg.mount,
            files: files,
        };
        ctx.add("config", &config);
    }
    // Image formatted using Display trait
    ctx.add("image", &format!("{}", dep.manifest.image.clone().unwrap()));

    // Ports exposed as is
    ctx.add("httpPort", &dep.manifest.httpPort);

    // Health check
    if let Some(ref h) = dep.manifest.health {
        ctx.add("health", h);
    }

    // Volume mounts
    ctx.add("volume_mounts", &dep.manifest.volume_mounts);

    // Init containers
    ctx.add("init_containers", &dep.manifest.init_containers);

    // Volumes
    ctx.add("volumes", &dep.manifest.volumes);

    // Temporary full manifest access - don't reach into this directly
    ctx.add("mf", &dep.manifest);

    Ok(ctx)
}

fn template_config(dep: &Deployment, mount: &ConfigMappedFile) -> Result<String> {
    let ctx = make_base_context(dep)?;
    Ok((dep.render)(&mount.name, &ctx)?)
}

use std::path::PathBuf;
use std::fs;
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
    /// Environment folder (one of: dev / qa / preprod / prod )
    pub environment: String,
    /// Location parameter (one of: uk )
    pub location: String,
    /// Manifest
    pub manifest: Manifest,
    /// Context bound template render function
    pub render: Box<Fn(&str, &Context) -> Result<(String)>>,
}
impl Deployment {
    pub fn check(&self) -> Result<()> {
        let name = self.manifest.name.clone();
        if self.service != name.unwrap() {
            bail!("manifest name does not match service name");
        }
        Ok(())
    }
}


pub fn deployment(dep: &Deployment, to_stdout: bool, to_file: bool) -> Result<String> {
    let ctx = make_full_deployment_context(dep)?;
    let res = if dep.manifest.disabled {
        warn!("Not generating yaml for disabled service");
        "---".into()
    } else {
        (dep.render)("deployment.yaml.j2", &ctx)?
    };
    if to_stdout {
        print!("{}", res);
    }
    if to_file {
        use std::path::Path;
        use std::fs::File;
        use std::io::prelude::*;

        let loc = Path::new(".");
        create_output(&loc.to_path_buf())?;
        let full_pth = loc.join("OUTPUT").join("values.yaml");
        let mut f = File::create(&full_pth)?;
        write!(f, "{}\n", res)?;
        info!("Wrote kubefiles for {} in {}", dep.service, full_pth.to_string_lossy());
    }
    Ok(res)
}
