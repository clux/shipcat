/// This file contains the `shipcat get` subcommand
use std::collections::BTreeMap;

use serde_json;
use super::{Result, Manifest, Config};


// ----------------------------------------------------------------------------
// Reducers from manifest::reducers

pub fn versions(conf: &Config, region: &str) -> Result<()> {
    let output = Manifest::get_versions(conf, region)?;
    print!("{}\n", serde_json::to_string_pretty(&output)?);
    Ok(())
}

pub fn images(conf: &Config, region: &str) -> Result<()> {
    let output = Manifest::get_images(conf, region)?;
    print!("{}\n", serde_json::to_string_pretty(&output)?);
    Ok(())
}

pub fn codeowners(conf: &Config, region: &str) -> Result<()> {
    let output = Manifest::get_codeowners(conf, region)?.join("\n");
    print!("{}\n", output);
    Ok(())
}

// ----------------------------------------------------------------------------
// Reducers for the Config

#[derive(Serialize)]
struct ClusterInfo {
    region: String,
    namespace: String,
    environment: String,
    apiserver: String,
    cluster: String,
    vault: String,
}

/// Entry point for clusterinfo
///
/// Need explicit region: shipcat get -r preprodca-green clusterinfo
pub fn clusterinfo(conf: &Config, ctx: &str) -> Result<()> {
    // a bit of magic here to work out region from context if given
    let (region, reg) = conf.get_region(ctx)?;
    // find the cluster serving the region (there's usually one, maybe a fallover)

    // if the kube context is a literal cluster name (as created by tarmak)
    // then find the associated cluster by looking up conf.clusters:
    let (cname, cluster) = if let Some(r) = conf.clusters.get(ctx) {
        (&region, r) // region == context name in this case
    } else {
        // otherwise: explicit context refers to a context served by exactly one cluster
        // e.g. dev-global1 inside kops-global1
        let candidates = conf.clusters.iter().filter(|(_k, v)| {
            v.regions.contains(&region)
        }).collect::<Vec<_>>();
        if candidates.len() != 1 {
            bail!("Ambiguous context {} must be served by exactly one cluster", ctx);
        }
        candidates[0]
    };
    let ci = ClusterInfo {
        region: region.clone(),
        namespace: reg.namespace,
        environment: reg.environment,
        apiserver: cluster.api.clone(),
        cluster: cname.clone(),
        vault: reg.vault.url.clone(),
    };
    print!("{}\n", serde_json::to_string_pretty(&ci)?);
    Ok(())
}


// ----------------------------------------------------------------------------
// hybrid reducers

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
            publiclyAccessible: !api.internal,
        });
    }

    let output = APIStatusOutput{environment, services};
    print!("{}\n", serde_json::to_string_pretty(&output)?);
    Ok(())
}
