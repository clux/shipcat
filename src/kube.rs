use std::collections::HashMap;

use tera::{Tera, Context};
use super::{Result};
use super::template::render;
use super::manifest::*;

#[derive(Serialize, Clone, Default)]
pub struct Mount {
    pub name: String,
    pub value: String,
}

fn template_config(tera: &Tera, name: &str, mount: &ConfigMount, env: &str) -> Result<String> {
    // friendly env name (used by newrelic config)
    // TODO: should be dev-uk or dev once namespace changes
    let envmap: HashMap<&str, &str> =[
        ("dev", "development"), // dev env has descriptive name development
    ].iter().cloned().collect();

    // newrelic api key for dev
    // TODO: generalize
    let license = "007015786e56e693643ba29dcc4e59aee5e0ca42".to_string();

    // currenly a reusable context for the various templated configs
    let mut ctx = Context::new();

    ctx.add("newrelic_license", &license); // for newrelic
    ctx.add("app", &name.to_string()); // for newrelic
    ctx.add("environment", envmap.get(env).unwrap()); // for newrelic
    Ok(render(tera, &mount.name, &ctx)?)

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


pub fn generate(tera: &Tera, mf: Manifest, env: &str, to_stdout: bool, to_file: bool) -> Result<String> {
    let mut context = Context::new();
    context.add("mf", &mf);

    // hm, any other version probably needs it passed in...
    let tagmap: HashMap<&str, &str> =[
        ("dev", "develop"), // dev env uses develop docker tags
    ].iter().cloned().collect();
    context.add("tag", tagmap.get(env).unwrap());

    if let Some(h) = mf.health {
        context.add("boottime", &h.wait.to_string());
    } else {
        context.add("boottime", &"30".to_string());
    }
    context.add("ports", &mf.ports);
    context.add("healthPort", &mf.ports[0]); // TODO: health check proper

    let mut mounts : Vec<Mount> = vec![];
    for mount in mf.config.iter() {
        let res = template_config(tera, &mf.name, mount, env)?;
        mounts.push(Mount { name: mount.dest.clone(), value: res });
    }
    context.add("mounts", &mounts);

    let res = render(tera, "deployment.yaml", &context)?;
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
        info!("Wrote kubefiles for {} in {}", mf.name, full_pth.to_string_lossy());
    }
    Ok(res)
}


#[allow(unused_variables)]
pub fn ship(tera: &Tera, mf: &Manifest) -> Result<()> {
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
