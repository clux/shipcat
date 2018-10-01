/// This file contains the `shipcat get` subcommand
use std::io::{self, Write};
use std::collections::BTreeMap;
use semver::Version;
use serde_json;
use super::{Result, Manifest, Config};

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
    let res = serde_json::to_string_pretty(&output)?;
    let _ = io::stdout().write(&format!("{}\n", res).as_bytes());
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
    let res = serde_json::to_string_pretty(&output)?;
    let _ = io::stdout().write(&format!("{}\n", res).as_bytes());
    Ok(())
}

/// Generate CODEOWNERS github syntax for services
pub fn codeowners(conf: &Config, region: &str) -> Result<()> {
    let services = Manifest::available()?;
    let mut output = vec![];

    for svc in services {
        let mf = Manifest::stubbed(&svc, &conf, &region)?;
        if let Some(md) = mf.metadata {
            let mut ghids = vec![];
            // unwraps guaranteed by validates on Manifest and Config
            let owners = &conf.teams.iter().find(|t| t.name == md.team).unwrap().owners;
            for o in owners.clone() {
                ghids.push(format!("@{}", o.github.unwrap()));
            }
            if !owners.is_empty() {
                output.push(format!("services/{}/* {}", mf.name, ghids.join(" ")));
            }
        }
    }
    let res = output.join("\n");
    let _ = io::stdout().write(&format!("{}\n", res).as_bytes());
    Ok(())
}

#[derive(Serialize)]
struct ClusterInfo {
    region: String,
    namespace: String,
    environment: String,
    apiserver: String,
    cluster: String,
}
pub fn clusterinfo(conf: &Config, ctx: &str) -> Result<()> {
    // a bit of magic here to work out region from context if given
    let (region, reg) = conf.get_region(ctx)?;
    // find the cluster serving the region (guaranteed only one by Config::verify)
    let (cname, cluster) = conf.clusters.iter().find(|(_k, v)| {
        v.regions.contains(&region)
    }).expect(&format!("region {} is served by a cluster", region));
    // print out all the relevant data
    let ci = ClusterInfo {
        region: region,
        namespace: reg.namespace,
        environment: reg.environment,
        apiserver: cluster.api.clone(),
        cluster: cname.clone(),
    };
    let res = serde_json::to_string_pretty(&ci)?;
    let _ = io::stdout().write(&format!("{}\n", res).as_bytes());
    Ok(())
}


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
    services_suffix: String,
    base_url: String,
    ip_whitelist: Vec<String>,
}
pub fn apistatus(conf: &Config, region: &str) -> Result<()> {
    let all_services = Manifest::available()?;
    let mut services = BTreeMap::new();
    let reg = conf.regions[region].clone();

    // Get Environment Config
    let environment = EnvironmentInfo {
        name: region.to_string(),
        services_suffix: reg.kong.base_url,
        base_url: reg.base_urls.get("services").unwrap_or(&"".to_owned()).to_string(),
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
    for (name, api) in reg.kong.extra_apis {
        services.insert(name, APIServiceParams {
            uris: api.uris.unwrap_or("".into()),
            hosts: api.hosts.unwrap_or("".into()),
            publiclyAccessible: false,
        });
    }

    let output = APIStatusOutput{environment, services};
    let res = serde_json::to_string_pretty(&output)?;
    let _ = io::stdout().write(&format!("{}\n", res).as_bytes());
    Ok(())
}
