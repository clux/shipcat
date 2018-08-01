/// This file contains the `shipcat get` subcommand
use std::collections::BTreeMap;
use semver::Version;
use serde_json;
use super::{Result, Manifest, Config};

#[derive(Serialize)]
struct APIStatusOutput {
    environment: EnvironmentInfo,
    services: BTreeMap<String, APIServiceParams>,
}

#[derive(Serialize)]
struct APIServiceParams {
    hosts: String,
    uris: String,
    publiclyAccessible: bool,
}

#[derive(Serialize)]
struct EnvironmentInfo {
    name: String,
    base_services: String,
    ip_whitelist: Vec<String>,
}

pub fn versions(conf: &Config, region: &str) -> Result<()> {
    let services = Manifest::available()?;
    let mut output : BTreeMap<String, Version> = BTreeMap::new();

    for svc in services {
        let mf = Manifest::stubbed(&svc, &conf, &region)?;
        if mf.regions.contains(&region.to_string()) {
            if let Some(v) = mf.version {
                if let Ok(sv) = Version::parse(&v) {
                    output.insert(svc, sv);
                }
            }
        }
    }
    let _ = println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

pub fn images(conf: &Config, region: &str) -> Result<()> {
    let services = Manifest::available()?;
    let mut output : BTreeMap<String, String> = BTreeMap::new();

    for svc in services {
        let mf = Manifest::stubbed(&svc, &conf, &region)?;
        if mf.regions.contains(&region.to_string()) {
            if let Some(i) = mf.image {
                output.insert(svc, i);
            }
        }
    }
    let _ = println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

pub fn apistatus(conf: &Config, region: &str) -> Result<()> {
    let all_services = Manifest::available()?;
    let mut services = BTreeMap::new();
    let reg = conf.regions[region].clone();

    // Get Environment Config
    let environment = EnvironmentInfo {
        name: region.to_string(),
        base_services: reg.kong.base_url,
        ip_whitelist: reg.ip_whitelist,
    };

    // Get API Info from Manifests
    for svc in all_services {
        let mf = Manifest::stubbed(&svc, &conf, &region)?;
        if mf.regions.contains(&region.to_string()) {
            if let Some(k) = mf.kong {            
                services.insert(svc, APIServiceParams {
                    uris: k.uris.unwrap_or("".into()),
                    hosts: k.hosts.unwrap_or("".into()),
                    publiclyAccessible: mf.publiclyAccessible, 
                });
            }
        }
    }

    // Get extra API Info from Config
    for (name, api) in reg.kong.extra_apis.clone() {
        services.insert(name, APIServiceParams {
            uris: api.uris.unwrap_or("".into()),
            hosts: api.hosts.unwrap_or("".into()),
            publiclyAccessible: false, 
        });
    }

    let output = APIStatusOutput{environment, services};
    let _ = println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
