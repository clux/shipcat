use std::collections::HashMap;

use tera::{Tera, Context};
use super::{Result};
use super::manifest::*;

/*fn newrelic(tera: &Tera, mf: &Manifest, env: &str) -> Result<String> {
    let envmap: HashMap<&str, &str> =[
        ("dev", "development"), // dev env has descriptive name development
    ].iter().cloned().collect();

    let env = envmap.get(env).unwrap().to_string();
    let license = "234l23eenistr983255342y".to_string(); // TODO: vault
    let mut ctx = Context::new();
    ctx.add("license_key", &license);
    ctx.add("app", &mf.name);
    ctx.add("environment", &env);
    Ok(tera.render("newrelic-java.yml", &ctx)?)
}
*/

#[derive(Serialize, Clone, Default)]
pub struct Mount {
    pub name: String,
    pub value: String,
}

fn template_config(tera: &Tera, name: &str, mount: &ConfigMount, env: &str) -> Result<String> {
    // friendly env name (used by newrelic config)
    let envmap: HashMap<&str, &str> =[
        ("dev", "development"), // dev env has descriptive name development
    ].iter().cloned().collect();

    // newrelic api key for dev
    // TODO: generalize
    let license = "007015786e56e693643ba29dcc4e59aee5e0ca42".to_string();

    // currenly a reusable context for the various templated configs
    let mut ctx = Context::new();

    ctx.add("license_key", &license); // for newrelic
    ctx.add("app", &name.to_string()); // for newrelic
    ctx.add("environment", envmap.get(env).unwrap()); // for newrelic
    Ok(tera.render(&mount.name, &ctx)?)

}

pub fn generate(tera: &Tera, mf: Manifest, env: &str, to_stdout: bool) -> Result<String> {
    let mut context = Context::new();
    context.add("mf", &mf);

    // hm, any other version probably needs it passed in...
    let tagmap: HashMap<&str, &str> =[
        ("dev", "develop"), // dev env uses develop docker tags
    ].iter().cloned().collect();
    context.add("tag", tagmap.get(env).unwrap());

    if !mf.ports.is_empty() {
        context.add("ports", &mf.ports);
        context.add("healthPort", &mf.ports[0]); // TODO: health check proper
    }
    let mut mounts : Vec<Mount> = vec![];
    for mount in mf.config.iter() {
        let res = template_config(tera, &mf.name, mount, env)?;
        mounts.push(Mount { name: mount.dest.clone(), value: res });
    }
    context.add("mounts", &mounts);

    let res = tera.render("deployment.yaml", &context)?;
    if to_stdout {
        print!("{}", res);
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
