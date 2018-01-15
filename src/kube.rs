// kube api structs are go style came case:
#![allow(non_snake_case)]

/*#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Metadata {
    name: Option<String>,
    namespace: Option<String>,
    labels: Option<String>
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ServiceAccount {
    pub apiVersion: String,
    pub kind: String,
    pub metadata: Metadata,
}


#[derive(Serialize, Deserialize, Clone, Default)]
pub struct DeploymentStrategy {
}
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct DeploymentTemplate {
}



#[derive(Serialize, Deserialize, Clone, Default)]
pub struct DeploymentSpec {
    replicas: u32,
    minReadySeconds: u32,
    strategy: DeploymentStrategy,
    template: DeploymentTemplate,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Deployment {
    pub apiVersion: String,
    pub kind: String,
    pub metadata: Metadata,
    pub spec: DeploymentSpec,
}

impl Deployment {
    pub fn new() -> Deployment {
        Deployment {
            apiVersion: "extensions/v1beta1".into(),
            kind: "Deployment".into()
        }
    }
}
*/
//shelving explicit structs for templating for now

//use tera;

use std::env;
use std::fs::File;
use std::io::prelude::*;

use tera::{Tera, Context};
use super::{Manifest, Result};


pub fn generate() -> Result<String> {
    let mf = Manifest::read()?;

    let cfg_dir = env::current_dir()?.join("configs"); // TODO: config dir
    let tpl_path = cfg_dir.join("deployment.yaml");
    let mut f = File::open(&tpl_path)?;
    let mut tpl = String::new();
    f.read_to_string(&mut tpl)?;

    let mut context = Context::new();
    context.add("mf", &mf);
    let res = Tera::one_off(&tpl, &context, false).unwrap(); // TODO: convert to Error
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
