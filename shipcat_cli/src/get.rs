/// This file contains the `shipcat get` subcommand
use std::collections::BTreeMap;
use semver::Version;
use shipcat_definitions::Environment;
use shipcat_definitions::teams::ServiceOwnership;

use super::{Config, Team, Region, Result};


// ----------------------------------------------------------------------------
// Simple reducers

/// Find the hardcoded versions of services in a region
///
/// Services without a hardcoded version are not returned.
pub fn versions(conf: &Config, region: &Region) -> Result<BTreeMap<String, Version>> {
    let mut output = BTreeMap::new();
    for mf in shipcat_filebacked::available(conf, region)? {
        if let Some(v) = mf.version {
            if let Ok(sv) = Version::parse(&v) {
                output.insert(mf.base.name, sv);
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
    for mf in shipcat_filebacked::available(conf, region)? {
        if let Some(i) = mf.image {
            output.insert(mf.base.name, i);
        }
    }
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(output)
}

/// Generate codeowner strings for each service based based on team owners + admins
///
/// Cross references config.teams with manifest.metadata.team
/// Each returned string is Github CODEOWNER syntax
pub fn codeowners(conf: &Config) -> Result<Vec<String>> {
    let mut output = vec![];
    let org = &conf.github.organisation;
    for mf in shipcat_filebacked::all(conf)? {
        let md = mf.metadata;
        let mut ghids = vec![];
        match conf.serviceOwnership {
            // Deprecated dual mode:
            ServiceOwnership::SquadsOrLegacyTeam => {
                if let Some(s) = conf.owners.squads.get(&md.team) {
                    // Check teams.yml for squads first:
                    if let Some(gha) = &s.github.admins {
                        ghids.push(format!("@{}/{}", org.to_lowercase(), gha));
                    }
                    // Add all squad members. Helpful because github codeowners are bad for teams
                    // (Teams need to be added explicitly to the repo...)
                    // Can perhaps be removed in the future
                    for o in &s.members {
                        if let Some(p) = conf.owners.people.get(o) {
                            ghids.push(format!("@{}", p.github));
                        }
                    }
                } else if let Some(t) = &conf.teams.iter().find(|t| t.name == md.team) {
                    // Otherwise legacy team mode
                    if let Some(gha) = &t.githubAdmins {
                        ghids.push(format!("@{}/{}", org.to_lowercase(), gha));
                    }
                    for o in t.owners.clone() {
                        ghids.push(format!("@{}", o.github.unwrap()));
                    }
                } else {
                    warn!("No team found for {} in shipcat.conf - ignoring {}", md.team, mf.name);
                }
            }
            ServiceOwnership::Squads => {
                if let Some(s) = conf.owners.squads.get(&md.team) {
                    if let Some(gha) = &s.github.admins {
                        ghids.push(format!("@{}/{}", org.to_lowercase(), gha));
                    }
                    // Add all squad members. Helpful because github codeowners are bad for teams
                    // (Teams need to be added explicitly to the repo...)
                    // Can perhaps be removed in the future
                    for o in &s.members {
                        if let Some(p) = conf.owners.people.get(o) {
                            ghids.push(format!("@{}", p.github));
                        }
                    }
                } else {
                    warn!("No squad found for {} in teams.yml - ignoring {}", md.team, mf.name);
                }
            },
        }
        if !ghids.is_empty() {
            output.push(format!("services/{}/* {}", mf.name, ghids.join(" ")));
        }

    }
    println!("{}", output.join("\n"));
    Ok(output)
}

/// Generate vault policies based on team admins of services
///
/// Cross refereneces config.teams with manifest.metadata.team
/// The output is the same across all regions to avoid chicken-egg problems.
/// Introducing services to dev first, where dev vault section is open solves this.
///
/// Usage:
/// shipcat get vaultpolicy teamname | vault policy write github-team-name -
/// vault write auth/github/map/teams/github-team-name value=github-team-name
///
/// Assumes you have setup github provider using right organisation.
/// vault write auth/github/config organization={GithubOrganisation}
pub fn vaultpolicy(conf: &Config, region: &Region, team_name: &str) -> Result<String> {
    let mfs = shipcat_filebacked::all(conf)?;
    let team = match conf.serviceOwnership {
        ServiceOwnership::SquadsOrLegacyTeam => {
            if let Some(s) = conf.owners.squads.get(team_name) {
                if s.github.admins.is_none() {
                    warn!("Squad '{}' does not define a github.admins team in teams.yml", s.name);
                }
                s.name.clone()
            }
            else if let Some(t) = conf.teams.iter().find(|t| t.name == team_name) {
                // fallback to legacy teams
                if t.githubAdmins.is_none() {
                    warn!("Team '{}' does not define a githubAdmins team in shipcat.conf", t.name);
                }
                t.name.clone()
            } else {
                bail!("Team '{}' does not exist in shipcat.conf", team_name);
            }
        },
        ServiceOwnership::Squads => {
            if let Some(s) = conf.owners.squads.get(team_name) {
                if s.github.admins.is_none() {
                    warn!("Squad '{}' does not define a github.admins team in teams.yml", s.name);
                }
                s.name.clone()
            } else {
                bail!("Squad '{}' does not exist in teams.yml", team_name)
            }
        }
    };
    let output = region.vault.make_policy(mfs, &team, region.environment.clone())?;
    println!("{}", output);
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kong: Option<String>,
    // TODO: this optional
    pub vault: String,
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
        environment: reg.environment.to_string(),
        apiserver: clust.api,
        cluster: clust.name,
        vault: reg.vault.url.clone(),
        kong: reg.kong.map(|k| k.config_url),
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
    region: RegionInfo,
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
struct RegionInfo {
    name: String,
    environment: Environment,
    base_urls: BTreeMap<String, String>,
    ip_whitelist: Vec<String>,
}
pub fn apistatus(conf: &Config, reg: &Region) -> Result<()> {
    let mut services = BTreeMap::new();

    // Get Environment Config
    let region = RegionInfo {
        name: reg.name.clone(),
        environment: reg.environment.clone(),
        base_urls: reg.base_urls.clone(),
        ip_whitelist: reg.ip_whitelist.clone(),
    };

    // Get API Info from Manifests
    for svc in shipcat_filebacked::available(conf, reg)? {
        let mf = shipcat_filebacked::load_manifest(&svc.base.name, &conf, &reg)?;
        for k in mf.kongApis {
            let mut params = APIServiceParams {
                uris: k.uris.unwrap_or("".into()),
                hosts: k.hosts.join(","),
                internal: k.internal,
                publiclyAccessible: mf.publiclyAccessible,
                websockets: false,
            };
            if let Some(g) = &mf.gate {
                // `manifest.verify` ensures that if there is a gate conf,
                // `gate.public` must be equal to `publiclyAccessible`.
                // That means that the following line does not alter the value
                // of `params.publiclyAccessible` but will be useful during the
                // migration of manifest configuration (ie deprecate
                // `publiclyAccessible` in favour of `gate.public`).
                params.publiclyAccessible = g.public;
                params.websockets = g.websockets;
            }
            services.insert(k.name, params);
        }
    }

    // Get extra API Info from Config: TODO: remove
    if let Some(kong) = &reg.kong {
        for (name, api) in kong.extra_apis.clone() {
            services.insert(name, APIServiceParams {
                uris: api.uris.unwrap_or("".into()),
                hosts: api.hosts.join(","),
                internal: api.internal,
                publiclyAccessible: api.publiclyAccessible,
                // TODO [DIP-499]: `extra_apis` do not support `gate` confs
                websockets: false,
            });
        }
    }

    let output = APIStatusOutput{region, services};
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

// ----------------------------------------------------------------------------


use super::structs::ResourceRequirements;
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
    let mut bd = ResourceBreakdown::new(conf.teams.clone()); // zero for all the things

    let mut sum : ResourceRequirements<f64> = Default::default();
    let mut extra : ResourceRequirements<f64> = Default::default(); // autoscaling limits

    for svc in shipcat_filebacked::available(conf, region)? {
        let mf = shipcat_filebacked::load_manifest(&svc.base.name, conf, region)?;
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

/// ResourceRequirements for all regions
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
