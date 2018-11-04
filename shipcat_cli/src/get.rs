/// This file contains the `shipcat get` subcommand
use std::collections::BTreeMap;

use super::{Result, Manifest, Config, Backend, Region};


// ----------------------------------------------------------------------------
// Reducers from manifest::reducers

pub fn versions(region: &Region) -> Result<()> {
    let output = Manifest::get_versions(region)?;
    print!("{}\n", serde_json::to_string_pretty(&output)?);
    Ok(())
}

pub fn images(conf: &Config, region: &Region) -> Result<()> {
    let output = Manifest::get_images(&conf.defaults, region)?;
    print!("{}\n", serde_json::to_string_pretty(&output)?);
    Ok(())
}

pub fn codeowners(conf: &Config, region: &Region) -> Result<()> {
    let output = Manifest::get_codeowners(&conf.teams, region)?.join("\n");
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
    // TODO: these two optional
    vault: String,
    kong: String,
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
        kong: reg.kong.config_url.clone(),
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
pub fn apistatus(conf: &Config, reg: &Region) -> Result<()> {
    let mut services = BTreeMap::new();

    // Get Environment Config
    let environment = EnvironmentInfo {
        name: reg.name.clone(),
        services_suffix: reg.kong.base_url.clone(),
        base_url: reg.base_urls.get("services").unwrap_or(&"".to_owned()).to_string(),
        ip_whitelist: reg.ip_whitelist.clone(),
    };

    // Get API Info from Manifests
    for svc in Manifest::available(&reg.name)? {
        let mf = Manifest::stubbed(&svc, &conf.defaults, &reg)?;
        if let Some(k) = mf.kong {
            services.insert(svc, APIServiceParams {
                uris: k.uris.unwrap_or("".into()),
                hosts: k.hosts.unwrap_or("".into()),
                internal: k.internal,
                publiclyAccessible: mf.publiclyAccessible,
            });
        }
    }

    // Get extra API Info from Config
    for (name, api) in reg.kong.extra_apis.clone() {
        services.insert(name, APIServiceParams {
            uris: api.uris.unwrap_or("".into()),
            hosts: api.hosts.unwrap_or("".into()),
            internal: api.internal,
            publiclyAccessible: api.publiclyAccessible,
        });
    }

    let output = APIStatusOutput{environment, services};
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

// ----------------------------------------------------------------------------

use shipcat_definitions::ResourceBreakdown;

/// Resource use for a single region
pub fn resources(conf: &Config, region: &Region) -> Result<()> {
    let bd = Manifest::resources(&conf.teams, region)?.normalise();
    println!("{}", serde_json::to_string_pretty(&bd)?);
    Ok(())
}

/// Resources for all regions
pub fn totalresources(conf: &Config) -> Result<()> {
    let mut bd = ResourceBreakdown::new(conf.teams.clone()); // zero for all the things
    for r in conf.list_regions() {
        let (_, reg) = conf.get_region(&r)?;
        let res = Manifest::resources(&conf.teams, &reg)?;
        bd.totals.base += res.totals.base;
        bd.totals.extra += res.totals.extra;
        for t in &conf.teams {
            let rhs = &res.teams[&t.name];
            let e = bd.teams.get_mut(&t.name).unwrap(); // exists by ResourceBreakdown::new
            e.base += rhs.base.clone();
            e.extra += rhs.extra.clone();
        }
    }
    bd = bd.normalise();
    println!("{}", serde_json::to_string_pretty(&bd)?);
    Ok(())
}
