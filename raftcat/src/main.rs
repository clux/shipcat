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
#[macro_use] extern crate tera;
#[macro_use] extern crate failure;

use kubernetes::client::APIClient;
use shipcat_definitions::{Manifest, Config};

mod kube;
use kube::{ManifestMap, Result};

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
    if let Some(mf) = req.state().safe.lock().unwrap().get_manifest(name)? {
        let pretty = serde_yaml::to_string(&mf)?;
        let mfstub = mf.clone().stub(&region).unwrap();
        let pretty_stub = serde_yaml::to_string(&mfstub)?;
        let mut ctx = tera::Context::new();
        ctx.insert("manifest", &mf);
        ctx.insert("pretty_manifest", &pretty);
        ctx.insert("pretty_manifest_stub", &pretty);
        ctx.insert("region", &region);

        // TODO externalise:
        let logzio_account = "46609";
        let logzio_link = format!("https://app-eu.logz.io/#/dashboard/kibana/dashboard/{app}-{env}?accountIds={account_id}",
          app = &mf.name, env = &region.name, account_id = &logzio_account);

        // TODO externalise:
        let vault_ui_base_url = "https://vault.babylontech.co.uk/secrets/generic/secret";
        let vault_link = format!("{vault_ui_base_url}/{env}/{app}/",
          vault_ui_base_url = &vault_ui_base_url, app = &mf.name, env = &region.name);

        // TODO externalise
        let sentry_base_url = "https://dev-uk-sentry.ops.babylontech.co.uk/sentry";
        // TODO: get through Sentry API
        let sentry_project_slug = "core-ruby";
        let sentry_link = format!("{sentry_base_url}/{sentry_project_slug}",
          sentry_base_url = &sentry_base_url, sentry_project_slug = &sentry_project_slug);

        // TODO externalise
        let grafana_base_url = "https://dev-grafana.ops.babylontech.co.uk/d/oHzT4g0iz/kubernetes-services";
        let grafana_link = format!("{grafana_base_url}?var-cluster={cluster}&var-namespace={namespace}&var-deployment={app}",
          grafana_base_url = &grafana_base_url,
          app = &mf.name,
          cluster = &cluster.name,
          namespace = &region.namespace);

        ctx.insert("vault_link", &vault_link);
        ctx.insert("logzio_link", &logzio_link);
        ctx.insert("sentry_link", &sentry_link);
        ctx.insert("grafana_link", &grafana_link);
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
