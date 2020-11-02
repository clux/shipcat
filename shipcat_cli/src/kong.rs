use std::{
    collections::BTreeMap,
    io::{self, Write},
};

use super::{
    structs::{
        kongfig::{kongfig_apis, kongfig_consumers, Api, Certificate, Consumer, Plugin, Upstream},
        Kong,
    },
    Config, KongConfig, Region, Result,
};

/// KongOutput matches the format expected by the Kong Configurator script
#[derive(Serialize)]
pub struct KongOutput {
    pub apis: BTreeMap<String, Kong>,
    pub kong: KongConfig,
}

/// KongOutput for Kongfig
#[derive(Serialize)]
pub struct KongfigOutput {
    pub host: String,
    pub headers: Vec<String>,
    pub apis: Vec<Api>,
    pub consumers: Vec<Consumer>,
    pub plugins: Vec<Plugin>,
    pub upstreams: Vec<Upstream>,
    pub certificates: Vec<Certificate>,
}

impl KongfigOutput {
    pub fn new(data: KongOutput, region: &Region) -> Self {
        KongfigOutput {
            host: data.kong.clone().config_url,
            headers: vec![],
            apis: kongfig_apis(data.apis, data.kong.clone(), region),
            consumers: kongfig_consumers(data.kong),
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
    name: String,
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

pub async fn generate_kong_output(conf: &Config, region: &Region) -> Result<KongOutput> {
    let mut apis = BTreeMap::new();
    if let Some(kong) = &region.kong {
        // Generate list of APIs to feed to Kong
        for mf in shipcat_filebacked::available(conf, region).await? {
            debug!("Scanning service {:?}", mf);
            for k in mf.kong_apis {
                if let Some(clash) = apis.insert(k.name.clone(), k) {
                    bail!("A Kong API named {:?} is already defined", clash.name);
                }
            }
        }

        // Add general Kong region config
        for (name, api) in kong.extra_apis.clone() {
            if let Some(clash) = apis.insert(name, api) {
                bail!("A Kong API named {:?} is already defined", clash.name);
            }
        }
        Ok(KongOutput {
            apis,
            kong: kong.clone(),
        })
    } else {
        bail!("kong not available in {}", region.name)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum KongOutputMode {
    /// Kongfig CRD - TODO:
    Crd,
    /// Kongfig raw yaml
    Kongfig,
}

/// Generate Kong config from a filled in global config
pub async fn output(conf: &Config, region: &Region, mode: KongOutputMode) -> Result<()> {
    let data = generate_kong_output(conf, &region).await?;
    let output = match mode {
        KongOutputMode::Crd => {
            let res = KongCrdOutput::new(&region.name, data);
            serde_yaml::to_string(&res)?
        }
        KongOutputMode::Kongfig => {
            let res = KongfigOutput::new(data, region);
            serde_yaml::to_string(&res)?
        }
    };
    let _ = io::stdout().write(format!("{}\n", output).as_bytes());
    Ok(())
}

/// Return the config_url for the given region
pub fn config_url(region: &Region) -> Result<()> {
    if let Some(k) = &region.kong {
        println!("{}", k.config_url);
    } else {
        bail!("No kong specified in {} region", region.name);
    }
    Ok(())
}
