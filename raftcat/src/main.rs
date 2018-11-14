#![allow(unused_imports, unused_variables)]

#[macro_use] extern crate serde;
extern crate serde_json;
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
#[macro_use] extern crate failure;

use kubernetes::client::APIClient;
use shipcat_definitions::Manifest;

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
    pub fn get_manifests(&mut self) -> Result<ManifestMap> {
        self.maybe_update_cache()?;
        Ok(self.manifests.clone())
    }
}

use std::sync::{Arc, Mutex};
#[derive(Clone)]
struct StateSafe {
    pub safe: Arc<Mutex<AppState>>
}
impl StateSafe {
    pub fn new(client: APIClient) -> Self {
        StateSafe { safe: Arc::new(Mutex::new(AppState::new(client))) }
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

fn health(_: &HttpRequest<StateSafe>) -> HttpResponse {
    HttpResponse::Ok().json("healthy")
}

fn main() -> Result<()> {
    use kubernetes::config::{self, Configuration};
    sentry::integrations::panic::register_panic_handler();
    let dsn = std::env::var("SENTRY_DSN").expect("Sentry DSN required");
    let _guard = sentry::init(dsn); // must keep _guard in scope

    std::env::set_var("RUST_LOG", "actix_web=debug,raftcat=debug");
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
            .middleware(middleware::Logger::default().exclude("/health"))
            .middleware(sentry_actix::SentryMiddleware::new())
            .resource("/manifests/{name}/resources", |r| r.method(Method::GET).f(get_resource_usage))
            .resource("/manifests/{name}", |r| r.method(Method::GET).f(get_single_manifest))
            .resource("/manifests", |r| r.method(Method::GET).f(get_all_manifests))
            .resource("/health", |r| r.method(Method::GET).f(health))
        })
        .bind("0.0.0.0:8080").expect("Can not bind to 0.0.0.0:8080")
        .shutdown_timeout(0)    // <- Set shutdown timeout to 0 seconds (default 60s)
        .start();

    info!("Starting listening on 0.0.0.0:8080");
    let _ = sys.run();
    std::process::exit(0); // SIGTERM ends up here eventually
}
