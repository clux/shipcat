use tera::{Tera, Context};
use super::{Manifest, Result};


fn newrelic(tera: &Tera, mf: &Manifest) -> Result<String> {
    let env = "development".to_string(); // TODO: from CLI
    let license = "234l23eenistr983255342y".to_string(); // TODO: vault
    let mut ctx = Context::new();
    ctx.add("license_key", &license);
    ctx.add("app", &mf.name);
    ctx.add("environment", &env);
    Ok(tera.render("newrelic-python.yml", &ctx)?)
}

pub fn generate(tera: &Tera) -> Result<String> {
    let mut mf = Manifest::read()?;
    mf.fill()?;

    let newrelic_cfg = newrelic(tera, &mf)?;

    let mut context = Context::new();
    context.add("mf", &mf);
    if !mf._portmaps.is_empty() {
        context.add("ports", &mf._portmaps);
        context.add("healthPort", &mf._portmaps[0].target); // TODO: health check proper
    }
    context.add("newrelic", &newrelic_cfg);
    let res = tera.render("deployment.yaml", &context)?;
    print!("{}", res);
    Ok(res)
}



// kubectl get pod -n dev -l=k8s-app=clinical-knowledge

// for full info: -o json - can grep that for stuff?


// kubectl describe pod -n dev -l=k8s-app=clinical-knowledge
// kubectl describe service -n dev -l=k8s-app=clinical-knowledge
// kubectl describe deployment -n dev -l=k8s-app=clinical-knowledge



// corresponding service account:
// kubectl describe serviceaccount -n dev clinical-knowledge
