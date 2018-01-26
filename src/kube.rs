use std::collections::HashMap;

use tera::Context; // just a hashmap wrapper
use super::{Result};
use super::manifest::*;

/// Completely filled in `ConfigMount`
#[derive(Serialize, Clone, Default)]
pub struct RenderedMount {
    pub name: String,
    pub path: String,
    pub configs: Vec<RenderedConfig>,
}
/// Completely filled in `ConfigMountedFile`
#[derive(Serialize, Clone, Default)]
pub struct RenderedConfig {
    pub name: String,
    pub value: String,
}


fn template_config(dep: &Deployment, mount: &ConfigMountedFile) -> Result<String> {
    // friendly env-loc name (used by newrelic config)
    let envloc = format!("{}-{}", dep.environment, dep.location);

    // newrelic api key for dev
    // TODO: generalize
    let license = "007015786e56e693643ba29dcc4e59aee5e0ca42".to_string();

    // currenly a reusable context for the various templated configs
    let mut ctx = Context::new();

    ctx.add("newrelic_license", &license); // for newrelic
    ctx.add("app", &dep.service); // for newrelic
    ctx.add("environmentlocation", &envloc); // for newrelic
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
        if self.service != self.manifest.name {
            bail!("manifest name does not match service name");
        }
        Ok(())
    }
}


pub fn generate(dep: &Deployment, to_stdout: bool, to_file: bool) -> Result<String> {
    let mut context = Context::new();
    context.add("mf", &dep.manifest);

    // hm, any other version probably needs it passed in...
    let tagmap: HashMap<&str, &str> =[
        ("dev", "develop"), // dev env uses develop docker tags
    ].iter().cloned().collect();
    context.add("tag", &tagmap[&*dep.environment]);

    if let Some(ref h) = dep.manifest.health {
        context.add("boottime", &h.wait.to_string());
    } else {
        context.add("boottime", &"30".to_string());
    }
    context.add("ports", &dep.manifest.ports);
    if !dep.manifest.ports.is_empty() {
        context.add("healthPort", &dep.manifest.ports[0]); // TODO: health check proper
    }
    let mut strategy = None;
    if let Some(ref rep) = dep.manifest.replicas {
        if rep.max != rep.min {
            strategy = Some("rolling".to_string());
        }
    }
    context.add("replication_strategy", &strategy);

    let mut mounts = vec![];
    for mount in dep.manifest.volumes.clone() {
        let mut files = vec![];
        for cfg in &mount.configs {
            let res = template_config(dep, cfg)?;
            files.push(RenderedConfig { name: cfg.dest.clone(), value: res });
        }
        mounts.push(RenderedMount {
            name: mount.name.unwrap(), // filled in by implicits
            path: mount.mount,
            configs: files,
        });
    }
    context.add("mounts", &mounts);

    let res = (dep.render)("deployment.yaml", &context)?;
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
