use serde_json;
use serde_yaml;
use std::io::{self, Write};
use std::collections::BTreeMap;

use super::{Manifest, Result, Config};
use super::structs::Kong;
use super::structs::kongfig::{kongfig_apis, kongfig_consumers};
use super::structs::kongfig::{Api, Consumer, Plugin, Upstream, Certificate};
use super::config::KongConfig;

/// KongOutput matches the format expected by the Kong Configurator script
#[derive(Serialize)]
struct KongOutput {
    pub apis: BTreeMap<String, Kong>,
    pub kong: KongConfig,
}

/// KongOutput for Kongfig
#[derive(Serialize, Deserialize)]
struct KongfigOutput {
    pub host: String,
    pub headers: Vec<String>,
    pub apis: Vec<Api>,
    pub consumers: Vec<Consumer>,
    pub plugins: Vec<Plugin>,
    pub upstreams: Vec<Upstream>,
    pub certificates: Vec<Certificate>
}

impl KongfigOutput {
    fn new(data: KongOutput) -> Self {
        KongfigOutput {
            host: data.kong.clone().config_url.into(),
            headers: vec![],
            apis: kongfig_apis(data.apis, data.kong.clone()),
            consumers: kongfig_consumers(data.kong.clone()),
            plugins: vec![],
            upstreams: vec![],
            certificates: vec![],
        }
    }
}

/// KongOutput in CRD form
#[derive(Serialize)]
struct KongCrdOutput {
    pub apiVersion: String,
    pub kind: String,
    pub metadata: Metadata,
    pub spec: KongOutput,
}
#[derive(Serialize)]
struct Metadata {
    name: String
}
impl KongCrdOutput {
    fn new(region: &str, data: KongOutput) -> Self {
        KongCrdOutput {
            apiVersion: "shipcat.babylontech.co.uk/v1".into(),
            kind: "KongConfig".into(),
            metadata: Metadata {
                name: format!("shipcat-kong-{}", region),
            },
            spec: data,
        }
    }
}

fn generate_kong_output(conf: &Config, region: &str) -> Result<KongOutput> {
    let mut apis = BTreeMap::new();

    // Generate list of APIs to feed to Kong
    for svc in Manifest::available()? {
        debug!("Scanning service {:?}", svc);
        let mf = Manifest::stubbed(&svc, conf, region)?; // does not need secrets
        if !mf.disabled && mf.regions.contains(&region.to_string()) {
            debug!("Found service {} in region {}", mf.name, region);
            if let Some(k) = mf.kong {
                apis.insert(svc, k);
            }
        }
    }

    // Add general Kong region config
    let reg = conf.regions[region].clone();
    for (name, api) in reg.kong.extra_apis.clone() {
        apis.insert(name, api);
    }
    Ok(KongOutput { apis, kong: reg.kong })
}

#[derive(Serialize, Deserialize, Debug)]
pub enum KongOutputMode {
    Json,
    Crd,
    Kongfig,
}

/// Generate Kong config from a filled in global config
pub fn output(conf: &Config, region: &str, mode: KongOutputMode) -> Result<()> {
    let data = generate_kong_output(conf, &region)?;
    let output = match mode {
        KongOutputMode::Json => {
            serde_json::to_string_pretty(&data)?
        },
        KongOutputMode::Crd => {
            let res = KongCrdOutput::new(region, data);
            serde_yaml::to_string(&res)?
        },
        KongOutputMode::Kongfig => {
            let res = KongfigOutput::new(data);
            serde_yaml::to_string(&res)?
        }
    };
    let _ = io::stdout().write(format!("{}\n", output).as_bytes());

    Ok(())
}

/// Return the config_url for the given region
pub fn config_url(conf: &Config, region: &str) -> Result<()> {
    let reg = conf.regions[&region.to_string()].clone();
    println!("{}", reg.kong.config_url);
    Ok(())
}

pub fn reconcile(conf: &Config, region: &str) -> Result<()> {
    use std::env;
    use std::path::Path;
    use std::fs::File;
    use std::io::{Write};
    let reg = conf.regions[&region.to_string()].clone();

    let kong = generate_kong_output(&conf, region)?;
    let output = serde_json::to_string_pretty(&kong)?;

    // write kong-{region}.json
    let fname = format!("kong-{}.json", region);
    let pth = Path::new(".").join(&fname);
    debug!("Writing kong values for {} to {}", region, pth.display());
    let mut f = File::create(&pth)?;
    write!(f, "{}\n", output)?;
    debug!("Wrote kong values for {} to {}: \n{}", region, pth.display(), output);

    // guess kong-configurator location
    let kongpth = format!("{}/kong.py",
        env::var("KONG_CONFIGURATOR_DIR").unwrap_or("/kong-configurator".into())
    );
    // python3 /kong-configurator/kong.py -c kong-{region}.json -u $KONG_URL
    use std::process::Command;
    let args = vec![kongpth, "-c".into(), fname, "-u".into(), reg.kong.config_url];
    info!("python3 {}", args.join(" "));
    let s = Command::new("python3").args(&args).status()?;
    if !s.success() {
        bail!("Subprocess failure from kong configurator: {}", s.code().unwrap_or(1001))
    }
    Ok(())
}

pub fn kongfig_reconcile(conf: &Config, region: &str) -> Result<()> {
    use std::path::Path;
    use std::fs::File;
    use std::io::{Write};
    let reg = conf.regions[&region.to_string()].clone();

    let data = generate_kong_output(&conf, region)?;
    let res = KongfigOutput::new(data);
    let output = serde_yaml::to_string(&res)?;

    // write kong-{region}.yaml
    let fname = format!("kong-{}.yaml", region);
    let pth = Path::new(".").join(&fname);
    debug!("Writing kongfig values for {} to {}", region, pth.display());
    let mut f = File::create(&pth)?;
    write!(f, "{}\n", output)?;
    debug!("Wrote kongfig values for {} to {}: \n{}", region, pth.display(), output);

    // As it happens, we can only write the file from here, as we can't run
    // `kongfig` from inside kubecat (we don't want to pull in node/npm, or do
    // docker-in-docker).

    // TODO later: pass this data to a CRD, and reconcile in-cluster! 🚀

    // FYI, this is the actual reconcile command:
    //
    // docker run -t
    //   -v $PWD:/volume quay.io/babylonhealth/kubecat:kongfig
    //   kongfig apply
    //   --host kong-admin-uk.dev.babylontech.co.uk
    //   --path /volume/kong-{region}.yaml
    //   --https # or not

    let v = reg.kong.config_url.split("://").collect::<Vec<_>>();
    assert_eq!(v.len(), 2);
    let (protocol, host) = (v[0], v[1]);
    let mut args = vec![
        "docker".into(), "run".into(),
        "-v".into(), "$PWD:/volume".into(),
        "quay.io/babylonhealth/kubecat:kongfig".into(),
        "kongfig".into(), "apply".into(),
        "--host".into(), host.into(),
        "--path".into(), format!("/volume/{}", fname)
    ];
    if protocol == "https" {
        args.push("--https".into());
    }
    info!("Reconcile with: sudo {}", args.join(" "));
    //let s = Command::new("sudo").args(&args).status()?;
    //if !s.success() {
    //    bail!("Subprocess failure from kong configurator: {}", s.code().unwrap_or(1001))
    //}
    Ok(())
}
