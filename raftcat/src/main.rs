#![allow(unused_imports, unused_variables)]

#[macro_use] extern crate serde;
extern crate serde_json;
extern crate serde_yaml;
#[macro_use] extern crate serde_derive;
extern crate url;
extern crate http;
#[macro_use] extern crate log;
extern crate env_logger; // TODO: slog + slog-json
extern crate kubernetes;
extern crate shipcat_definitions;
extern crate sentry;
extern crate sentry_actix;
extern crate actix;
extern crate actix_web;
extern crate reqwest;
extern crate semver;
#[macro_use] extern crate tera;
#[macro_use] extern crate failure;
extern crate chrono;

use kubernetes::client::APIClient;
use shipcat_definitions::{Manifest, Config};

mod kube;
use kube::{ManifestMap, Result};

use chrono::Local;

// ----------------------------------------------------------------------------------
// Web server interface
use actix_web::{server, App, Path, Responder, HttpRequest, HttpResponse, middleware};
use actix_web::http::{header, Method, StatusCode};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// State shared between http requests
#[derive(Clone)]
struct AppState {
    pub client: APIClient,
    pub manifests: ManifestMap,
    last_update: Instant,
}
impl AppState {
    pub fn new(client: APIClient) -> Self {
        info!("Loading state from CRDs");
        let manifests = kube::get_shipcat_manifests(&client).unwrap();
        AppState {
            client: client,
            manifests: manifests,
            last_update: Instant::now(),
        }
    }

    fn maybe_update_cache(&mut self) -> Result<()> {
        if self.last_update.elapsed().as_secs() > 120 {
            debug!("Refreshing state from CRDs");
            self.manifests = kube::get_shipcat_manifests(&self.client)?;
            self.last_update = Instant::now();
        }
        Ok(())
    }
    pub fn get_manifest(&mut self, key: &str) -> Result<Option<Manifest>> {
        self.maybe_update_cache()?;
        if let Some(mf) = self.manifests.get(key) {
            return Ok(Some(mf.spec.clone()));
        }
        Ok(None)
    }
    pub fn get_config(&mut self) -> Result<Config> {
        let name = "dev-uk"; // TODO: from env
        let cfg = kube::get_shipcat_config(&self.client, name)?;
        Ok(cfg.spec)
    }
    pub fn get_manifests(&mut self) -> Result<ManifestMap> {
        self.maybe_update_cache()?;
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
        let t = compile_templates!(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/*"));
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
fn get_service(req: &HttpRequest<StateSafe>) -> Result<HttpResponse> {
    let name = req.match_info().get("name").unwrap();
    let cfg = req.state().safe.lock().unwrap().get_config()?;
    let (cluster, region) = cfg.resolve_cluster("dev-uk").unwrap();
    if let Some(mf) = req.state().safe.lock().unwrap().get_manifest(name)?.clone() {
        let pretty = serde_yaml::to_string(&mf)?;
        let mfstub = mf.clone().stub(&region).unwrap();
        let pretty_stub = serde_yaml::to_string(&mfstub)?;

        let md = mf.metadata.clone().unwrap();
        let (vlink, version) = if let Some(ver) = kube::get_version(&mf.name).ok() { // fill in version from cluster info
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
        let (support, supportlink) = (md.support.clone(), md.support.unwrap().link());


        let mut ctx = tera::Context::new();
        ctx.insert("manifest", &mf);
        ctx.insert("pretty_manifest", &pretty);
        ctx.insert("pretty_manifest_stub", &pretty);
        ctx.insert("region", &region);
        ctx.insert("versionlink", &vlink);
        ctx.insert("version", &version);
        ctx.insert("health", &health);
        ctx.insert("support", &support);
        ctx.insert("supportlink", &supportlink);

        // TODO externalise:
        let logzio_base_url = "https://app-eu.logz.io/#/dashboard/kibana/dashboard";
        //TODO externalise
        let logzio_account = "46609";
        // TODO externalise
        let sentry_base_url = "https://dev-uk-sentry.ops.babylontech.co.uk/sentry";
        let sentry_project_slug = kube::get_sentry_slug(
            &region.sentry.clone().unwrap().url,
            &region.environment,
            &mf.name,
        ).unwrap_or(format!("PROJECT_NOT_FOUND"));
        // TODO: get through Sentry API
        //let sentry_project_slug = "core-ruby";
        // TODO externalise
        let grafana_base_url = "https://dev-grafana.ops.babylontech.co.uk/d/oHzT4g0iz/kubernetes-services";

        let logzio_link = format!("{logzio_base_url}/{app}-{env}?accountIds={account_id}",
          logzio_base_url = &logzio_base_url, app = &mf.name, env = &region.name, account_id = &logzio_account);

        let sentry_link = format!("{sentry_base_url}/{sentry_project_slug}",
          sentry_base_url = &sentry_base_url, sentry_project_slug = &sentry_project_slug);

        let grafana_link = format!("{grafana_base_url}?var-cluster={cluster}&var-namespace={namespace}&var-deployment={app}",
          grafana_base_url = &grafana_base_url,
          app = &mf.name,
          cluster = &cluster.name,
          namespace = &region.namespace);

        let date = Local::now();
        let time = format!("{now}", now = date.format("%Y-%m-%d %H:%M:%S"));

        ctx.insert("vault_link", &region.vault_url(&mf.name));
        ctx.insert("logzio_link", &region.logzio_url(&mf.name));
        ctx.insert("sentry_link", &sentry_link);
        ctx.insert("grafana_link", &region.grafana_url(&mf.name, &cluster.name));
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

fn index(req: &HttpRequest<StateSafe>) -> Result<HttpResponse> {
    let mut ctx = tera::Context::new();
    ctx.insert("guts", "::<>");
    let t = req.state().template.lock().unwrap();
    let s = t.render("index.tera", &ctx).unwrap(); // TODO: map error
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

fn main() -> Result<()> {
    use kubernetes::config::{self, Configuration};
    sentry::integrations::panic::register_panic_handler();
    //let dsn = std::env::var("SENTRY_DSN").expect("Sentry DSN required");
    //let _guard = sentry::init(dsn); // must keep _guard in scope

    std::env::set_var("RUST_LOG", "actix_web=info,raftcat=debug");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    // Load the config: local kube config prioritised first for local development
    // NB: Only supports a config with client certs locally (e.g. kops setup)
    let cfg = config::load_kube_config().unwrap_or_else(|_| {
        config::incluster_config().expect("in cluster config failed to load")
    });

    let client = APIClient::new(cfg);
    let state = StateSafe::new(client);

    info!("Creating http server");
    let sys = actix::System::new("raftcat");
    server::new(move || {
        App::with_state(state.clone())
            .handler("/static", actix_web::fs::StaticFiles::new(concat!(env!("CARGO_MANIFEST_DIR"), "/static")).unwrap())
            .middleware(middleware::Logger::default().exclude("/health"))
            .middleware(sentry_actix::SentryMiddleware::new())
            .resource("/config", |r| r.method(Method::GET).f(get_config))
            .resource("/manifests/{name}/resources", |r| r.method(Method::GET).f(get_resource_usage))
            .resource("/manifests/{name}", |r| r.method(Method::GET).f(get_single_manifest))
            .resource("/manifests", |r| r.method(Method::GET).f(get_all_manifests))
            .resource("/service/{name}", |r| r.method(Method::GET).f(get_service))
            .resource("/health", |r| r.method(Method::GET).f(health))
            .resource("/", |r| r.method(Method::GET).f(index))
        })
        .bind("0.0.0.0:8080").expect("Can not bind to 0.0.0.0:8080")
        .shutdown_timeout(0)    // <- Set shutdown timeout to 0 seconds (default 60s)
        .start();

    info!("Starting listening on 0.0.0.0:8080");
    let _ = sys.run();
    std::process::exit(0); // SIGTERM ends up here eventually
}
