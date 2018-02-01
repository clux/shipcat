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
    ctx.add("ports", &dep.manifest.ports);

    // ugly heath check stuff - TODO: fix this
    if let Some(ref h) = dep.manifest.health {
        ctx.add("boottime", &h.wait.to_string());
    } else {
        ctx.add("boottime", &"30".to_string());
    }
    if !dep.manifest.ports.is_empty() {
        ctx.add("healthPort", &dep.manifest.ports[0]); // TODO: health check proper
    }

    // ugly replication strategy hack - basically pointless
    let mut strategy = None;
    if let Some(ref rep) = dep.manifest.replicas {
        if rep.max != rep.min {
            strategy = Some("rolling".to_string());
        }
    }
    ctx.add("replication_strategy", &strategy);

    // Init containers
    ctx.add("init_containers", &dep.manifest.init_containers);

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


pub fn generate(dep: &Deployment, to_stdout: bool, to_file: bool) -> Result<String> {
    let ctx = make_full_deployment_context(dep)?;
    let res = (dep.render)("deployment.yaml.j2", &ctx)?;
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


#[allow(unused_variables)]
pub fn ship(dep: &Deployment, mf: &Manifest) -> Result<()> {
    //let kubefile = generate(tera, mf)?;
    // TODO: write kubefile
    // TODO: kubectl apply -f kubefile
    unimplemented!()
}
// kubectl get pod -n dev -l=k8s-app=clinical-knowledge

// for full info: -o json - can grep that for stuff?


// kubectl describe pod -n dev -l=k8s-app=clinical-knowledge
// kubectl describe service -n dev -l=k8s-app=clinical-knowledge
// kubectl describe deployment -n dev -l=k8s-app=clinical-knowledge



// corresponding service account:
// kubectl describe serviceaccount -n dev clinical-knowledge
