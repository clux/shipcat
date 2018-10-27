/// This file contains the `shipcat get` subcommand
use shipcat_definitions::math::ResourceTotals;
use std::collections::BTreeMap;

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
    internal: bool,
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
                    internal: k.internal,
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
            internal: api.internal,
            publiclyAccessible: api.publiclyAccessible,
        });
    }

    let output = APIStatusOutput{environment, services};
    print!("{}\n", serde_json::to_string_pretty(&output)?);
    Ok(())
}

// ----------------------------------------------------------------------------
/*
use super::structs::Resources;
use shipcat_definitions::manifest::math::ResourceTotals;

#[derive(Serialize, Default)]
pub struct ResourceBreakdown {
    /// Total totals
    pub totals: ResourceTotals,
    /// A partition of totals info teams
    pub teams: BTreeMap<String, ResourceTotals>,
}


pub fn resources(conf: &Config, region: &str) -> Result<()> {
    let services = Manifest::available()?;
    let mut bd = ResourceBreakdown::default(); // zero for all the things

    let mut sum : Resources<f64> = Resources::default();
    let mut extra : Resources<f64> = Resources::default(); // autoscaling limits

    for svc in services {
        let mf = Manifest::basic(&svc, &conf, None)?;
        if mf.external || !mf.regions.contains(&region.to_string()) {
            continue; // only care about kube services in the current region
        }
        if let Some(ref md) = mf.metadata {
            let ResourceTotals { base: sb, extra: se } = mf.compute_resource_totals()?;
            sum += sb.clone();
            extra += se.clone();
            bd.teams.entry(md.team.clone())
                .and_modify(|e| {
                    e.base += sb.clone();
                    e.extra += se.clone();
                })
                .or_insert_with(|| {
                    ResourceTotals { base: sb, extra: se }
                }
            );
        } else {
            bail!("{} service does not have resources specification and metadata", mf.name)
        }
    }
    // convert gigs for all teams
    for (_, mut teamtot) in &mut bd.teams {
        teamtot.base.round();
        teamtot.extra.round();
    }
    // overall totals:
    sum.round();
    extra.round();
    bd.totals.base = sum;
    bd.totals.extra = extra;

    print!("{}\n", serde_yaml::to_string(&bd)?);
    Ok(())
}

pub fn totalresources(conf: &Config) -> Result<()> {
    let services = Manifest::available()?;
    let mut bd = ResourceBreakdown::default(); // zero for all the things

    let mut sum : Resources<f64> = Resources::default();
    let mut extra : Resources<f64> = Resources::default(); // autoscaling limits

    for svc in services {
        let tmpmf = Manifest::basic(&svc, &conf, None)?;
        if tmpmf.external {
            continue; // only care about kube services
        }
        for region in tmpmf.regions {
            let mf = Manifest::mocked(&svc, &conf, &region)?;
            if let Some(ref md) = mf.metadata {
                let ResourceTotals { base: sb, extra: se } = mf.compute_resource_totals()?;
                sum += sb.clone();
                extra += se.clone();
                bd.teams.entry(md.team.clone())
                    .and_modify(|e| {
                        e.base += sb.clone();
                        e.extra += se.clone();
                    })
                    .or_insert_with(|| {
                        ResourceTotals { base: sb, extra: se }
                    }
                );
            } else {
                bail!("{} service does not have resources specification and metadata", mf.name)
            }
        }
    }
    // convert gigs for all teams
    for (_, mut teamtot) in &mut bd.teams {
        teamtot.base.round();
        teamtot.extra.round();
    }
    // overall totals:
    sum.round();
    extra.round();
    bd.totals.base = sum;
    bd.totals.extra = extra;

    print!("{}\n", serde_yaml::to_string(&bd)?);
    Ok(())
}
*/
