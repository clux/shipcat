#![allow(unused_imports, unused_variables)]
#[macro_use] extern crate log;

use serde_derive::Serialize;
use std::{
    collections::{BTreeMap, HashMap},
    io,
};

use chrono::Local;
use reqwest::Url;
use shipcat_definitions::Manifest;
use std::env;

pub use raftcat::*;

fn find_team(owners: &Owners, slug: &str) -> Option<Squad> {
    owners.squads.get(slug).cloned()
}

// ----------------------------------------------------------------------------------
// Web server interface
use actix_files as fs;
use actix_web::{
    middleware,
    web::{self, Data},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};

// Route entrypoints
async fn get_single_manifest(c: Data<State>, req: HttpRequest) -> Result<HttpResponse> {
    let name = req.match_info().get("name").unwrap();
    if let Some(mf) = c.get_manifest(name).await? {
        Ok(HttpResponse::Ok().json(mf.spec))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
async fn get_all_manifests(c: Data<State>, _req: HttpRequest) -> Result<HttpResponse> {
    let mfs: BTreeMap<String, Manifest> = c.get_manifests().await?;
    Ok(HttpResponse::Ok().json(mfs))
}
async fn get_resource_usage(c: Data<State>, req: HttpRequest) -> Result<HttpResponse> {
    let name = req.match_info().get("name").unwrap();
    if let Some(mf) = c.get_manifest(name).await? {
        let totals = mf.spec.compute_resource_totals().unwrap(); // TODO: use 'failure' in shipcat_definitions
        Ok(HttpResponse::Ok().json(totals))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
async fn get_manifests_for_team(c: Data<State>, req: HttpRequest) -> Result<HttpResponse> {
    let name = req.match_info().get("name").unwrap();
    let cfg = c.get_config().await?;
    if let Some(t) = find_team(&cfg.owners, name) {
        let mfs = c.get_manifests_for(&t.name).await?;
        Ok(HttpResponse::Ok().json(mfs))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
async fn get_teams(c: Data<State>) -> Result<HttpResponse> {
    let cfg = c.get_config().await?;
    Ok(HttpResponse::Ok().json(cfg.owners.squads))
}

async fn get_versions(c: Data<State>) -> Result<HttpResponse> {
    let vers = c.get_versions().await?;
    Ok(HttpResponse::Ok().json(vers))
}

async fn get_kompass_hub_services(c: Data<State>, req: HttpRequest) -> Result<HttpResponse> {
    let manifests = c.get_manifests().await?;
    let mut services = HashMap::new();
    let path = match req.headers().get("host") {
        Some(host) => host.to_str().expect("host"),
        None => return Ok(HttpResponse::InternalServerError().finish()),
    };
    for (name, m) in manifests {
        services.insert(name, kompass::to_protobuf(m, &path));
    }
    Ok(HttpResponse::Ok().json(protos::services::ServicesResponse {
        services,
        ..Default::default()
    }))
}

async fn get_service(c: Data<State>, req: HttpRequest) -> Result<HttpResponse> {
    let name = req.match_info().get("name").unwrap();
    let cfg = c.get_config().await?;
    let region = c.get_region().await?;

    let revdeps = c.get_reverse_deps(name).await.ok();
    let newrelic_link = c.get_newrelic_link(name);
    let sentry_slug = c.get_sentry_slug(name);

    if let Some(mfobj) = c.get_manifest(name).await? {
        let mf = mfobj.spec;
        let pretty = serde_yaml::to_string(&mf)?;
        let mfstub = mf.clone().stub(&region).await.unwrap();

        let md = mf.metadata.clone().unwrap();
        let version = mf.version.clone().unwrap();
        let vlink = md.github_link_for_version(&version);

        let health = if let Some(h) = mf.health.clone() {
            h.uri
        } else if let Some(h) = mf.readinessProbe.clone() {
            serde_json::to_string(&h)?
        } else {
            // can be here if no exposed port
            "non-service".into()
        };
        let (support, supportlink) = (md.support.clone(), md.support.unwrap().link(&cfg.slack));
        // TODO: org in config
        let circlelink = format!("https://circleci.com/gh/babylonhealth/{}", mf.name);
        let quaylink = format!("https://{}/?tab=tags", mf.image.clone().unwrap());

        let (team, teamlink) = (md.team.clone(), format!("/raftcat/teams/{}", &md.team));
        // TODO: runbook

        let mut ctx = tera::Context::new();
        ctx.insert("raftcat", env!("CARGO_PKG_VERSION"));
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
        ctx.insert("mfenvstub", &mfstub.env);
        ctx.insert("mfdeps", &mf.dependencies);

        if let Some(status) = mfobj.status {
            let conds = &status.conditions;
            let mut cvec = vec![];
            if let Some(g) = &conds.generated {
                cvec.push(format!("Generated: {}", g.html_list_item().unwrap()));
            }
            if let Some(a) = &conds.applied {
                cvec.push(format!("Applied: {}", a.html_list_item().unwrap()));
            }
            if let Some(r) = &conds.rolledout {
                cvec.push(format!("RolledOut: {}", r.html_list_item().unwrap()));
            }
            ctx.insert("conditions", &cvec);
        }

        // integration insert if found in the big query
        if let Some(lio_link) = region.logzio_url(&mf.name) {
            ctx.insert("logzio_link", &lio_link);
        }
        if let Some(gf_link) = region.grafana_url(&mf.name) {
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

        // stats
        if let Ok(_usage) = mf.compute_resource_totals() {
            let usagen = _usage.normalise();
            ctx.insert("usage", &serde_json::to_string_pretty(&usagen)?);
            ctx.insert("cost", &usagen.daily_cost());
            ctx.insert("rollouts", &mf.estimate_rollout_iterations());
        }
        if let Some(ru) = mf.rollingUpdate {
            ctx.insert("rollingUpdate", &serde_json::to_string_pretty(&ru)?);
        }

        ctx.insert("revdeps", &revdeps);

        let date = Local::now();
        let time = date.format("%Y-%m-%d %H:%M:%S").to_string();

        ctx.insert("time", &time);
        let s = c.render_template("service.tera", ctx);
        Ok(HttpResponse::Ok().content_type("text/html").body(s))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}

async fn health() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json("healthy"))
}

async fn get_config(c: Data<State>, _req: HttpRequest) -> Result<HttpResponse> {
    let cfg = c.get_config().await?;
    Ok(HttpResponse::Ok().json(cfg))
}

#[derive(Serialize)]
struct SimpleManifest {
    name: String,
    team: String,
}
#[derive(Serialize)]
struct SimpleRegion {
    name: String,
    url: String,
}

async fn index(c: Data<State>, _req: HttpRequest) -> Result<HttpResponse> {
    let mut ctx = tera::Context::new();
    let data = c
        .get_manifests()
        .await?
        .into_iter()
        .map(|(k, m)| SimpleManifest {
            name: k,
            team: m.metadata.unwrap().team.to_lowercase(),
        })
        .collect::<Vec<_>>();
    let data = serde_json::to_string(&data)?;
    ctx.insert("manifests", &data);

    let regions = c
        .get_config()
        .await?
        .get_regions()
        .into_iter()
        .filter_map(|r| r.raftcat_url().map(|url| SimpleRegion { name: r.name, url }))
        .collect::<Vec<_>>();
    if regions.len() > 1 {
        let regions = serde_json::to_string(&regions)?;
        ctx.insert("regions", &regions);
    }

    ctx.insert("raftcat", env!("CARGO_PKG_VERSION"));
    let s = c.render_template("index.tera", ctx);
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    //sentry::integrations::panic::register_panic_handler();
    //let dsn = env::var("SENTRY_DSN").expect("Sentry DSN required");
    //let _guard = sentry::init(dsn); // must keep _guard in scope

    env::set_var("RUST_LOG", "actix_web=info,raftcat=info,kube=info");
    if let Ok(level) = env::var("LOG_LEVEL") {
        if level.to_lowercase() == "debug" {
            env::set_var("RUST_LOG", "actix_web=info,raftcat=debug,kube=debug");
        }
    }
    // env::set_var("RUST_BACKTRACE", "full"); // <- don't! this spams logz.io, rely on sentry!
    env_logger::init();

    // TODO: fix so that this path isn't checked at all
    env::set_var("VAULT_TOKEN", "INVALID"); // needed because it happens super early..

    // Set up kube access + fetch initial state. Crashing on failure here.
    let cfg = if let Ok(c) = kube::config::incluster_config() {
        c
    } else {
        kube::config::load_kube_config()
            .await
            .expect("Failed to load kube config")
    };
    let shared_state = state::init(cfg).await.unwrap();

    let region = shared_state.get_region().await.unwrap();
    let region_str = format!("{}{}", region.raftcat_url().expect("raftcat url"), "kompass-hub");
    let region_url = Url::parse(&region_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, "invalid raftcat url"))?;
    let kompass_evar = env::var("KOMPASS_URL").expect("Need KOMPASS_URL evar");
    let kompass_url = Url::parse(&kompass_evar)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, "invalid kompass url"))?;
    tokio::spawn(kompass::register(kompass_url, region_url));

    info!("Starting listening on 0.0.0.0:8080");
    HttpServer::new(move || {
        App::new()
            .data(shared_state.clone())
            .wrap(
                middleware::Logger::default()
                    .exclude("/health")
                    .exclude("/raftcat/health")
                    .exclude("/favicon.ico")
                    .exclude("/raftcat/static/*.png")
                    .exclude("/raftcat/static/images/*.png"),
            )
            //.wrap(prometheus.clone())
            //.wrap(sentry_actix...)
            .service(fs::Files::new("/raftcat/static", "./raftcat/static").index_file("index.html"))
            .service(web::resource("/raftcat/config").route(web::get().to(get_config)))
            .service(
                web::resource("/raftcat/manifests/{name}/resources").route(web::get().to(get_resource_usage)),
            )
            .service(web::resource("/raftcat/manifests/{name}").route(web::get().to(get_single_manifest)))
            .service(web::resource("/raftcat/manifests").route(web::get().to(get_all_manifests)))
            .service(web::resource("/raftcat/services/{name}").route(web::get().to(get_service)))
            .service(web::resource("/raftcat/teams/{name}").route(web::get().to(get_manifests_for_team)))
            .service(web::resource("/raftcat/teams").route(web::get().to(get_teams)))
            .service(web::resource("/raftcat/health").route(web::get().to(health)))
            .service(web::resource("/raftcat/versions").route(web::get().to(get_versions)))
            .service(web::resource("/raftcat/kompass-hub").route(web::get().to(get_kompass_hub_services)))
            .service(web::resource("/health").route(web::get().to(health))) // redundancy
            .service(web::resource("/raftcat/").route(web::get().to(index)))
    })
    .bind("0.0.0.0:8080")
    .expect("Can not bind to 0.0.0.0:8080")
    .shutdown_timeout(0)
    .run()
    .await
}
