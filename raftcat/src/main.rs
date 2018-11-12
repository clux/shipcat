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
extern crate actix;
extern crate actix_web;
#[macro_use] extern crate failure;
use failure::{Error, Fail};

use kubernetes::client::APIClient;
use shipcat_definitions::{Crd, CrdList, Manifest};

type Result<T> = std::result::Result<T, Error>;

static GROUPNAME: &str = "babylontech.co.uk";
static SHIPCATRESOURCE: &str = "shipcatmanifests";

// Request builders
fn make_all_crd_entry_req(resource: &str, group: &str) -> Result<http::Request<Vec<u8>>> {
    let urlstr = format!("/apis/{group}/v1/{resource}?",
        group = group, resource = resource);
    let urlstr = url::form_urlencoded::Serializer::new(urlstr).finish();
    let mut req = http::Request::get(urlstr);
    req.body(vec![]).map_err(Error::from)
}
fn make_crd_entry_req(resource: &str, group: &str, name: &str) -> Result<http::Request<Vec<u8>>> {
    // TODO: namespace from evar
    let urlstr = format!("/apis/{group}/v1/namespaces/dev/{resource}/{name}?",
        group = group, resource = resource, name = name);
    let urlstr = url::form_urlencoded::Serializer::new(urlstr).finish();
    let mut req = http::Request::get(urlstr);
    req.body(vec![]).map_err(Error::from)
}
/*fn watch_crd_entry_after(resource: &str, group: &str, name: &str, rver: u32) -> Result<http::Request<Vec<u8>>> {
    let urlstr = format!("/apis/{group}/v1/namespaces/dev/{resource}/{name}?",
        group = group, resource = resource, name = name);
    let mut qp = url::form_urlencoded::Serializer::new(urlstr);

    qp.append_pair("timeoutSeconds", "30");
    qp.append_pair("watch", "true");

    // last version to watch after
    //qp.append_pair("resourceVersion", &rver.to_string());

    let urlstr = qp.finish();
    let mut req = http::Request::get(urlstr);
    req.body(vec![]).map_err(Error::from)
}*/

// program interface - request consumers
pub fn get_shipcat_manifests(client: &APIClient) -> Result<Vec<Crd<Manifest>>> {
    let req = make_all_crd_entry_req(SHIPCATRESOURCE, GROUPNAME)?;
    let res = client.request::<CrdList<Manifest>>(req)?;
    let mut found = vec![];
    for i in &res.items {
        found.push(i.spec.name.clone())
    }
    debug!("{}", found.join(", "));
    Ok(res.items)
}

/*this doesn't actually work...
pub fn watch_shipcat_manifest(client: &APIClient, name: &str, rver: u32) -> Result<Crd<Manifest>> {
    let req = watch_crd_entry_after(SHIPCATRESOURCE, GROUPNAME, name, rver)
        .expect("failed to define crd watch request");
    let res = client.request::<Crd<_>>(req)?;
    debug!("{}", &res.spec.name);
    Ok(res)
}*/

pub fn get_shipcat_manifest(client: &APIClient, name: &str) -> Result<Crd<Manifest>> {
    let req = make_crd_entry_req(SHIPCATRESOURCE, GROUPNAME, name)?;
    let res = client.request::<Crd<Manifest>>(req)?;
    debug!("got {}", &res.spec.name);
    // TODO: merge with version found in rolling env?
    Ok(res)
}

// Web server interface
use actix_web::{server, App, Path, Responder, HttpRequest, HttpResponse, middleware};
use actix_web::http::{header, Method, StatusCode};

// Share kube client between http requests
use std::sync::{Arc, Mutex};
struct AppState {
    client: Arc<Mutex<APIClient>>,
}

// Route entrypoints
fn get_single_manifest(req: &HttpRequest<AppState>) -> HttpResponse {
    let name = req.match_info().get("name").unwrap();
    let client = req.state().client.lock().unwrap();
    let mf = get_shipcat_manifest(&client, name).unwrap();
    // TODO: cache results for 5min in app state
    HttpResponse::Ok().json(mf)
}
fn get_all_manifests(req: &HttpRequest<AppState>) -> HttpResponse {
    let client = req.state().client.lock().unwrap();
    let mfs = get_shipcat_manifests(&client).unwrap();
    HttpResponse::Ok().json(mfs)
}

fn health(_: &HttpRequest<AppState>) -> HttpResponse {
    HttpResponse::Ok().json("healthy")
}

fn main() -> Result<()> {
    use kubernetes::config::{self, Configuration};
    std::env::set_var("RUST_LOG", "actix_web=debug,raftcat=debug");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();
    let sys = actix::System::new("raftcat");

    server::new(move || {
        // Load the config: local kube config prioritised first for local development
        // NB: Only supports a config with client certs locally (e.g. kops setup)
        let cfg = config::load_kube_config().unwrap_or_else(|_| {
            config::incluster_config().expect("in cluster config failed to load")
        });
        let client = APIClient::new(cfg);
        let state = AppState { client: Arc::new(Mutex::new(client)) };
        App::with_state(state)
            .middleware(middleware::Logger::default())
            .resource("/manifests/{name}", |r| r.method(Method::GET).f(get_single_manifest))
            .resource("/manifests/", |r| r.method(Method::GET).f(get_all_manifests))
            .resource("/health", |r| r.method(Method::GET).f(health))
        })
        .bind("0.0.0.0:8080").expect("Can not bind to 0.0.0.0:8080")
        .shutdown_timeout(0)    // <- Set shutdown timeout to 0 seconds (default 60s)
        .start();

    println!("Starting http server: 0.0.0.0:8080");
    let _ = sys.run();
    unreachable!();
}
