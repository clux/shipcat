#![allow(unused_imports, unused_variables)]
#[macro_use] extern crate log;
extern crate env_logger; // TODO: slog + slog-json

extern crate sentry;
extern crate sentry_actix;
extern crate actix;
extern crate actix_web;
extern crate semver;
#[macro_use] extern crate tera;
extern crate kubernetes;
extern crate chrono;

extern crate failure;
use failure::err_msg;

#[macro_use] extern crate serde_derive;

extern crate raftcat;
pub use raftcat::*;

use kubernetes::client::APIClient;
use kubernetes::config as config;

use std::collections::BTreeMap;
use std::env;
use chrono::Local;

// some slug helpers
fn team_slug(name: &str) -> String {
    name.to_lowercase().replace("/", "-").replace(" ", "_")
}
fn find_team(cfg: &Config, slug: &str) -> Option<Team> {
    cfg.teams.iter().find(|t| team_slug(&t.name) == slug).map(Clone::clone)
}


// ----------------------------------------------------------------------------------
// Web server interface
use actix_web::{server, App, Path, Responder, HttpRequest, HttpResponse, middleware};
use actix_web::http::{header, Method, StatusCode};
use std::time::Instant;

/// State shared between http requests
#[derive(Clone)]
struct AppState {
    pub client: APIClient,
    pub manifests: ManifestMap,
    pub config: Config,
    pub relics: RelicMap,
    pub sentries: SentryMap,
    region: String,
    last_update: Instant,
}
impl AppState {
    pub fn new(client: APIClient) -> Self {
        info!("Loading state from CRDs");
        let rname = env::var("REGION_NAME").expect("Need REGION_NAME evar (kube context)");
        let config = kube::get_shipcat_config(&client, &rname)
            .expect("Need to be able to read config CRD").spec;
        let mut res = AppState {
            client: client,
            manifests: BTreeMap::new(),
            config: config,
            region: rname,
            relics: BTreeMap::new(),
            sentries: BTreeMap::new(),
            last_update: Instant::now(),
        };
        res.maybe_update_cache(true).expect("Need to be able to read manifest CRDs");
        res.update_slow_cache().expect("Need to be able to update cache (at least partially)");
        res
    }

    fn maybe_update_cache(&mut self, force: bool) -> Result<()> {
        if self.last_update.elapsed().as_secs() > 30 || force {
            info!("Refreshing state from CRDs");
            self.manifests = kube::get_shipcat_manifests(&self.client)?;
            match version::get_all() {
                Ok(versions) => {
                    info!("Loaded {} versions", versions.len());
                    for (k, mf) in &mut self.manifests {
                        mf.version = versions.get(k).map(String::clone);
                    }
                }
                Err(e) => warn!("Unable to load versions. VERSION_URL set? {}", err_msg(e))
            }
            self.last_update = Instant::now();
        }
        Ok(())
    }
    fn update_slow_cache(&mut self) -> Result<()> {
        let (cluster, region) = self.config.resolve_cluster(&self.region).unwrap();
        if let Some(s) = region.sentry {
            match sentryapi::get_slugs(&s.url, &region.environment) {
                Ok(res) => {
                    self.sentries = res;
                    info!("Loaded {} sentry slugs", self.sentries.len());
                },
                Err(e) => warn!("Unable to load sentry slugs. SENTRY evars set? {}", err_msg(e)),
            }
        } else {
            warn!("No sentry url configured for this region");
        }
        match newrelic::get_links(&region.name) {
            Ok(res) => {
                self.relics = res;
                info!("Loaded {} newrelic links", self.relics.len());
            },
            Err(e) => warn!("Unable to load newrelic projects. NEWRELIC evars set? {}", err_msg(e)),
        }
        Ok(())
    }
    pub fn watch_manifests(&mut self) -> Result<()> {
        info!("Starting watch!");
        let res = kube::watch_for_shipcat_manifest_updates(&self.client, self.manifests.clone())?;
        info!("got res!");
        Ok(())
    }
    pub fn get_manifest(&mut self, key: &str) -> Result<Option<Manifest>> {
        self.maybe_update_cache(false)?;
        if let Some(mf) = self.manifests.get(key) {
            return Ok(Some(mf.clone()));
        }
        Ok(None)
    }
    pub fn get_manifests_for(&self, team: &str) -> Result<Vec<String>> {
        let mfs = self.manifests.iter()
            .filter(|(_k, mf)| mf.metadata.clone().unwrap().team == team)
            .map(|(_k, mf)| mf.name.clone()).collect();
        Ok(mfs)
    }
    pub fn get_reverse_deps(&self, service: &str) -> Result<Vec<String>> {
        let mut res = vec![];
        for (svc, mf) in &self.manifests {
            if mf.dependencies.iter().any(|d| d.name == service) {
                res.push(svc.clone())
            }
        }
        Ok(res)
    }
    pub fn get_cluster_region(&self) -> Result<(Cluster, Region)> {
        let (cluster, region) = self.config.resolve_cluster(&self.region).expect("could not resolve cluster");
        Ok((cluster, region))
    }
    pub fn get_config(&self) -> Result<Config> {
        Ok(self.config.clone())
    }
    pub fn get_manifests(&mut self) -> Result<ManifestMap> {
        self.maybe_update_cache(false)?;
        Ok(self.manifests.clone())
    }
}

use std::sync::{Arc, Mutex};
#[derive(Clone)]
struct StateSafe {
    pub safe: Arc<Mutex<AppState>>,
    pub template: Arc<Mutex<tera::Tera>>,
}
impl StateSafe {
    pub fn new(client: APIClient) -> Self {
        let t = compile_templates!(concat!("raftcat", "/templates/*"));
        StateSafe {
            safe: Arc::new(Mutex::new(AppState::new(client))),
            template: Arc::new(Mutex::new(t)),
        }
    }
}

// Route entrypoints
fn get_single_manifest(req: &HttpRequest<StateSafe>) -> Result<HttpResponse> {
    let name = req.match_info().get("name").unwrap();
    if let Some(mf) = req.state().safe.lock().unwrap().get_manifest(name)? {
        Ok(HttpResponse::Ok().json(mf))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
fn get_all_manifests(req: &HttpRequest<StateSafe>) -> Result<HttpResponse> {
    let mfs = req.state().safe.lock().unwrap().get_manifests()?;
    Ok(HttpResponse::Ok().json(mfs))
}
fn get_resource_usage(req: &HttpRequest<StateSafe>) -> Result<HttpResponse> {
    let name = req.match_info().get("name").unwrap();
    if let Some(mf) = req.state().safe.lock().unwrap().get_manifest(name)? {
        let totals = mf.compute_resource_totals().unwrap(); // TODO: use 'failure' in shipcat_definitions
        Ok(HttpResponse::Ok().json(totals))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
fn get_manifests_for_team(req: &HttpRequest<StateSafe>) -> Result<HttpResponse> {
    let name = req.match_info().get("name").unwrap();
    let cfg = req.state().safe.lock().unwrap().get_config()?;
    if let Some(t) = find_team(&cfg, name) {
        let mfs = req.state().safe.lock().unwrap().get_manifests_for(&t.name)?.clone();
        Ok(HttpResponse::Ok().json(mfs))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
fn get_teams(req: &HttpRequest<StateSafe>) -> Result<HttpResponse> {
    let cfg = req.state().safe.lock().unwrap().get_config()?;
    Ok(HttpResponse::Ok().json(cfg.teams.clone()))
}

fn get_service(req: &HttpRequest<StateSafe>) -> Result<HttpResponse> {
    let name = req.match_info().get("name").unwrap();
    let cfg = req.state().safe.lock().unwrap().get_config()?;
    let (cluster, region) = req.state().safe.lock().unwrap().get_cluster_region()?;

    let revdeps = req.state().safe.lock().unwrap().get_reverse_deps(name).ok();
    let newrelic_link = req.state().safe.lock().unwrap().relics.get(name).map(String::to_owned);
    let sentry_slug = req.state().safe.lock().unwrap().sentries.get(name).map(String::to_owned);

    if let Some(mf) = req.state().safe.lock().unwrap().get_manifest(name)?.clone() {
        let pretty = serde_yaml::to_string(&mf)?;
        let mfstub = mf.clone().stub(&region).unwrap();
        let pretty_stub = serde_yaml::to_string(&mfstub)?;

        let md = mf.metadata.clone().unwrap();
        let (vlink, version) = if let Some(ver) = mf.version.clone() {
            if semver::Version::parse(&ver).is_ok() {
                let tag = md.version_template(&ver).unwrap_or(ver.to_string());
                (format!("{}/releases/tag/{}", md.repo, tag), tag)
            } else {
                (format!("{}/commit/{}", md.repo, ver), ver)
            }
        } else {
            (format!("{}", md.repo), "rolling".into())
        };
        let health = if let Some(h) = mf.health.clone() {
            h.uri
        } else {
            // mandatory to have one of these!
            serde_json::to_string(&mf.readinessProbe.clone().unwrap())?
        };
        let (support, supportlink) = (md.support.clone(), md.support.unwrap().link(&cfg.slack));
        // TODO: org in config
        let circlelink = format!("https://circleci.com/gh/Babylonpartners/{}", mf.name);
        let quaylink = format!("https://{}/?tab=tags", mf.image.clone().unwrap());

        let env_vars = mf.env.clone();
        let deps = mf.dependencies.clone();

        let (team, teamlink) = (md.team.clone(), format!("/raftcat/teams/{}", team_slug(&md.team)));
        // TODO: runbook

        let mut ctx = tera::Context::new();
        ctx.insert("manifest", &mf);
        ctx.insert("pretty_manifest", &pretty);
        ctx.insert("pretty_manifest_stub", &pretty);
        ctx.insert("region", &region);
        ctx.insert("version_link", &vlink);
        ctx.insert("version", &version);
        ctx.insert("health", &health);
        ctx.insert("support", &support);
        ctx.insert("support_link", &supportlink);
        ctx.insert("circle_link", &circlelink);
        ctx.insert("quay_link", &quaylink);
        ctx.insert("team", &team);
        ctx.insert("team_link", &teamlink);
        ctx.insert("mfenv", &mf.env);
        ctx.insert("mfdeps", &mf.dependencies);

        // integration insert if found in the big query
        if let Some(lio_link) = region.logzio_url(&mf.name) {
            ctx.insert("logzio_link", &lio_link);
        }
        if let Some(gf_link) = region.grafana_url(&mf.name, &cluster.name) {
            ctx.insert("grafana_link", &gf_link);
        }
        ctx.insert("vault_link", &region.vault_url(&mf.name));
        if let Some(slug) = sentry_slug {
            if let Some(sentry_link) = region.sentry_url(&slug) {
                ctx.insert("sentry_link", &sentry_link);
            }
        }
        if let Some(nr) = newrelic_link {
            ctx.insert("newrelic_link", &nr);
        }

        ctx.insert("revdeps", &revdeps);

        let date = Local::now();
        let time = format!("{now}", now = date.format("%Y-%m-%d %H:%M:%S"));

        ctx.insert("time", &time);
        let t = req.state().template.lock().unwrap();
        let s = t.render("service.tera", &ctx).unwrap(); // TODO: map error
        Ok(HttpResponse::Ok().content_type("text/html").body(s))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}

fn health(_: &HttpRequest<StateSafe>) -> HttpResponse {
    HttpResponse::Ok().json("healthy")
}

fn get_config(req: &HttpRequest<StateSafe>) -> Result<HttpResponse> {
    let cfg = req.state().safe.lock().unwrap().get_config()?;
    Ok(HttpResponse::Ok().json(cfg))
}

#[derive(Serialize)]
struct SimpleManifest {
    name: String,
    team: String,
}

fn index(req: &HttpRequest<StateSafe>) -> Result<HttpResponse> {
    let mut ctx = tera::Context::new();
    let mfs = req.state().safe.lock().unwrap().get_manifests()?;
    let data = mfs.into_iter().map(|(_k, m)| {
        SimpleManifest {
            name: m.name,
            team: m.metadata.unwrap().team.to_lowercase(),
        }
    }).collect::<Vec<_>>();
    let data = serde_json::to_string(&data)?;
    ctx.insert("manifests", &data);
    let t = req.state().template.lock().unwrap();
    let s = t.render("index.tera", &ctx).unwrap(); // TODO: map error
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

fn watch_blah(req: &HttpRequest<StateSafe>) -> Result<HttpResponse> {
    req.state().safe.lock().unwrap().watch_manifests()?;
    Ok(HttpResponse::NotFound().finish())
}

fn main() -> Result<()> {
    sentry::integrations::panic::register_panic_handler();
    let dsn = env::var("SENTRY_DSN").expect("Sentry DSN required");
    let _guard = sentry::init(dsn); // must keep _guard in scope

    env::set_var("RUST_LOG", "actix_web=info,raftcat=info,kubernetes=info");
    //env::set_var("RUST_LOG", "actix_web=info,raftcat=debug");
    //env::set_var("RUST_BACKTRACE", "full");
    env_logger::init();

    // Load the config: local kube config prioritised first for local development
    // NB: Only supports a config with client certs locally (e.g. kops setup)
    let cfg = match env::var("HOME").expect("have HOME dir").as_ref() {
        "/root" => config::incluster_config(),
        _ => config::load_kube_config(),
    }.expect("Failed to load kube config");

    let client = APIClient::new(cfg);
    let state = StateSafe::new(client);

    info!("Creating http server");
    let sys = actix::System::new("raftcat");
    server::new(move || {
        App::with_state(state.clone())
            .middleware(middleware::Logger::default().exclude("/raftcat/health"))
            .middleware(sentry_actix::SentryMiddleware::new())
            .handler("/raftcat/static", actix_web::fs::StaticFiles::new("./raftcat/static").unwrap())
            .resource("/raftcat/config", |r| r.method(Method::GET).f(get_config))
            .resource("/raftcat/manifests/{name}/resources", |r| r.method(Method::GET).f(get_resource_usage))
            .resource("/raftcat/manifests/{name}", |r| r.method(Method::GET).f(get_single_manifest))
            .resource("/raftcat/manifests", |r| r.method(Method::GET).f(get_all_manifests))
            .resource("/raftcat/services/{name}", |r| r.method(Method::GET).f(get_service))
            .resource("/raftcat/teams/{name}", |r| r.method(Method::GET).f(get_manifests_for_team))
            .resource("/raftcat/teams", |r| r.method(Method::GET).f(get_teams))
            .resource("/raftcat/health", |r| r.method(Method::GET).f(health))
            .resource("/raftcat/blah", |r| r.method(Method::GET).f(watch_blah))
            .resource("/raftcat/", |r| r.method(Method::GET).f(index))
        })
        .bind("0.0.0.0:8080").expect("Can not bind to 0.0.0.0:8080")
        .shutdown_timeout(0)    // <- Set shutdown timeout to 0 seconds (default 60s)
        .start();

    info!("Starting listening on 0.0.0.0:8080");
    let _ = sys.run();
    std::process::exit(0); // SIGTERM ends up here eventually
}
