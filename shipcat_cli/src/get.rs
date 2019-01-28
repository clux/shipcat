/// This file contains the `shipcat get` subcommand
use std::collections::BTreeMap;
use semver::Version;

use crate::structs::{
    rds::Rds,
    elasticache::ElastiCache,
};
use super::{Config, Team, Region};
use super::{Result, Manifest};


// ----------------------------------------------------------------------------
// Simple reducers

/// Find the hardcoded versions of services in a region
///
/// Services without a hardcoded version are not returned.
pub fn versions(conf: &Config, region: &Region) -> Result<BTreeMap<String, Version>> {
    let mut output = BTreeMap::new();
    for svc in Manifest::available(&region.name)? {
        let mf = Manifest::simple(&svc, &conf, &region)?;
        if let Some(v) = mf.version {
            if let Ok(sv) = Version::parse(&v) {
                output.insert(svc, sv);
            }
        }
    }
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(output)
}

/// Find the hardcoded images of services in a region
///
/// Services without a hardcoded image will assume the shipcat.conf specific default
pub fn images(conf: &Config, region: &Region) -> Result<BTreeMap<String, String>> {
    let mut output = BTreeMap::new();
    for svc in Manifest::available(&region.name)? {
        // NB: needs > raw version of manifests because we need image implicits..
        let mf = Manifest::simple(&svc, &conf, &region)?;
        if let Some(i) = mf.image {
            output.insert(svc, i);
        }
    }
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(output)
}

/// Generate codeowner strings for each service based based on team owners
///
/// Cross references config.teams with manifest.metadata.team
/// Each returned string is Github CODEOWNER syntax
pub fn codeowners(conf: &Config) -> Result<Vec<String>> {
    let mut output = vec![];
    for svc in Manifest::all()? {
        // Can rely on blank here because metadata is a global property
        let mf = Manifest::blank(&svc)?;
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
    println!("{}", output.join("\n"));
    Ok(output)
}

// ----------------------------------------------------------------------------
// Reducers for the Config

#[derive(Serialize)]
pub struct ClusterInfo {
    pub region: String,
    pub namespace: String,
    pub environment: String,
    pub apiserver: String,
    pub cluster: String,
    // TODO: these two optional
    pub vault: String,
    pub kong: String,
}

/// Entry point for clusterinfo
///
/// Need explicit region: shipcat get -r preprodca-green clusterinfo
pub fn clusterinfo(conf: &Config, ctx: &str, cluster: Option<&str>) -> Result<ClusterInfo> {
    assert!(conf.has_all_regions()); // can't work with reduced configs
    let (clust, reg) = conf.resolve_cluster(ctx, cluster.map(String::from))?;
    let ci = ClusterInfo {
        region: reg.name,
        namespace: reg.namespace,
        environment: reg.environment,
        apiserver: clust.api,
        cluster: clust.name,
        vault: reg.vault.url.clone(),
        kong: reg.kong.config_url.clone(),
    };
    println!("{}", serde_json::to_string_pretty(&ci)?);
    Ok(ci)
}

/// Vault
///
/// Prints just the vault url for a region
/// Because this is invariant over a region
pub fn vault_url(region: &Region) -> Result<String> {
    let out = region.vault.url.clone();
    println!("{}", out);
    Ok(out)
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
    websockets: bool,
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
        let mf = Manifest::simple(&svc, &conf, &reg)?;
        if let Some(k) = mf.kong {
            let mut params = APIServiceParams {
                uris: k.uris.unwrap_or("".into()),
                hosts: k.hosts.unwrap_or("".into()),
                internal: k.internal,
                publiclyAccessible: mf.publiclyAccessible,
                websockets: false,
            };
            if let Some(g) = mf.gate {
                // `manifest.verify` ensures that if there is a gate conf,
                // `gate.public` must be equal to `publiclyAccessible`.
                // That means that the following line does not alter the value
                // of `params.publiclyAccessible` but will be useful during the
                // migration of manifest configuration (ie deprecate
                // `publiclyAccessible` in favour of `gate.public`).
                params.publiclyAccessible = g.public;
                params.websockets = g.websockets;
            }
            services.insert(svc, params);
        }
    }

    // Get extra API Info from Config
    for (name, api) in reg.kong.extra_apis.clone() {
        services.insert(name, APIServiceParams {
            uris: api.uris.unwrap_or("".into()),
            hosts: api.hosts.unwrap_or("".into()),
            internal: api.internal,
            publiclyAccessible: api.publiclyAccessible,
            // TODO [DIP-499]: `extra_apis` do not support `gate` confs
            websockets: false,
        });
    }

    let output = APIStatusOutput{environment, services};
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}


/// Find the RDS instances to be provisioned for a region
///
/// Reduces all manifests in a region and produces a list for a terraform component
/// to act on.
pub fn databases(conf: &Config, region: &Region) -> Result<Vec<Rds>> {
    let mut dbs = Vec::new();
    for svc in Manifest::available(&region.name)? {
        // NB: needs > raw version of manifests because we need image implicits..
        let mf = Manifest::simple(&svc, &conf, &region)?;
        if let Some(db) = mf.database {
            dbs.push(db);
        }
    }
    println!("{}", serde_yaml::to_string(&dbs)?);
    Ok(dbs)
}

/// Find the ElastiCache instances to be provisioned for a region
///
/// Reduces all manifests in a region and produces a list for a terraform component
/// to act on.
pub fn caches(conf: &Config, region: &Region) -> Result<Vec<ElastiCache>> {
    let mut caches = Vec::new();
    for svc in Manifest::available(&region.name)? {
        // NB: needs > raw version of manifests because we need image implicits..
        let mf = Manifest::simple(&svc, &conf, &region)?;
        if let Some(db) = mf.redis {
            caches.push(db);
        }
    }
    println!("{}", serde_yaml::to_string(&caches)?);
    Ok(caches)
}

// ----------------------------------------------------------------------------


use super::structs::Resources;
use shipcat_definitions::math::ResourceTotals;

/// Complete breakdown of resource usage in total, and split by team.
///
/// Normally this is computed by `Manifest::resources` for a region-wide total.
/// Looping over all regions is possible in the CLI.
#[derive(Serialize)]
pub struct ResourceBreakdown {
    /// Total totals
    pub totals: ResourceTotals,
    /// A partition of totals info teams
    pub teams: BTreeMap<String, ResourceTotals>,
}

impl ResourceBreakdown {
    /// Constructor to ensure all valid teams are filled in
    pub fn new(tx: Vec<Team>) -> ResourceBreakdown {
        let mut teams = BTreeMap::new();
        for t in tx {
            teams.insert(t.name, ResourceTotals::default());
        }
        ResourceBreakdown { teams, totals: ResourceTotals::default() }
    }

    /// Round all numbers to gigs and full cores (for all teams)
    pub fn normalise(mut self) -> Self {
        for tt in &mut self.teams.values_mut() {
            tt.base.round();
            tt.extra.round();
        }
        self.totals.base.round();
        self.totals.extra.round();
        self
    }
}


/// Compute resource usage for all available manifests in a region.
fn resources_region(conf: &Config, region: &Region) -> Result<ResourceBreakdown> {
    let services = Manifest::available(&region.name)?;
    let mut bd = ResourceBreakdown::new(conf.teams.clone()); // zero for all the things

    let mut sum : Resources<f64> = Default::default();
    let mut extra : Resources<f64> = Default::default(); // autoscaling limits

    for svc in services {
        let mf = Manifest::base(&svc, conf, region)?;
        if let Some(ref md) = mf.metadata {
            let ResourceTotals { base: sb, extra: se } = mf.compute_resource_totals()?;
            sum += sb.clone();
            extra += se.clone();
            let e = bd.teams.get_mut(&md.team).unwrap(); // exists by ResourceBreakdown::new
            e.base += sb.clone();
            e.extra += se.clone();
        } else {
            bail!("{} service does not have resources specification and metadata", mf.name)
        }
    }
    bd.totals.base = sum;
    bd.totals.extra = extra;
    Ok(bd)
}


/// Resource use for a single region
pub fn resources(conf: &Config, region: &Region) -> Result<()> {
    let bd = resources_region(&conf, region)?.normalise();
    println!("{}", serde_json::to_string_pretty(&bd)?);
    Ok(())
}

/// Resources for all regions
pub fn totalresources(conf: &Config) -> Result<()> {
    let mut bd = ResourceBreakdown::new(conf.teams.clone()); // zero for all the things
    for r in conf.list_regions() {
        let reg = conf.get_region(&r)?;
        let res = resources_region(&conf, &reg)?;
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
