#![allow(unused_imports, unused_variables)]

use log::{info, warn, error, debug, trace};
use serde_derive::Serialize;

use std::env;
use chrono::Local;

pub use raftcat::*;

// some slug helpers
fn team_slug(name: &str) -> String {
    name.to_lowercase().replace("/", "-").replace(" ", "_")
}
fn find_team(cfg: &Config, slug: &str) -> Option<Team> {
    cfg.teams.iter().find(|t| team_slug(&t.name) == slug).cloned()
}


// ----------------------------------------------------------------------------------
// Web server interface
use actix_web::{
    server, App, Path, Responder, HttpRequest, HttpResponse, middleware,
    http::{header, Method, StatusCode},
};


// Route entrypoints
fn get_single_manifest(req: &HttpRequest<State>) -> Result<HttpResponse> {
    let name = req.match_info().get("name").unwrap();
    if let Some(mf) = req.state().get_manifest(name)? {
        Ok(HttpResponse::Ok().json(mf))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
fn get_all_manifests(req: &HttpRequest<State>) -> Result<HttpResponse> {
    let mfs = req.state().get_manifests()?;
    Ok(HttpResponse::Ok().json(mfs))
}
fn get_resource_usage(req: &HttpRequest<State>) -> Result<HttpResponse> {
    let name = req.match_info().get("name").unwrap();
    if let Some(mf) = req.state().get_manifest(name)? {
        let totals = mf.compute_resource_totals().unwrap(); // TODO: use 'failure' in shipcat_definitions
        Ok(HttpResponse::Ok().json(totals))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
fn get_manifests_for_team(req: &HttpRequest<State>) -> Result<HttpResponse> {
    let name = req.match_info().get("name").unwrap();
    let cfg = req.state().get_config()?;
    if let Some(t) = find_team(&cfg, name) {
        let mfs = req.state().get_manifests_for(&t.name)?.clone();
        Ok(HttpResponse::Ok().json(mfs))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
fn get_teams(req: &HttpRequest<State>) -> Result<HttpResponse> {
    let cfg = req.state().get_config()?;
    Ok(HttpResponse::Ok().json(cfg.teams.clone()))
}

fn get_service(req: &HttpRequest<State>) -> Result<HttpResponse> {
    let name = req.match_info().get("name").unwrap();
    let cfg = req.state().get_config()?;
    let region = req.state().get_region()?;

    let revdeps = req.state().get_reverse_deps(name).ok();
    let newrelic_link = req.state().get_newrelic_link(name);
    let sentry_slug = req.state().get_sentry_slug(name);

    if let Some(mf) = req.state().get_manifest(name)?.clone() {
        let pretty = serde_yaml::to_string(&mf)?;
        let mfstub = mf.clone().stub(&region).unwrap();


        let md = mf.metadata.clone().unwrap();
        let (vlink, version) = if let Some(ver) = mf.version.clone() {
            if semver::Version::parse(&ver).is_ok() {
                let tag = md.version_template(&ver).unwrap_or(ver.to_string());
                (format!("{}/releases/tag/{}", md.repo, tag), tag)
            } else {
                (format!("{}/commit/{}", md.repo, ver), ver)
            }
        } else {
            (md.repo, "rolling".into())
        };

        let health = if let Some(h) = mf.health.clone() {
            h.uri
        } else if let Some(h) = mf.readinessProbe.clone() {
            serde_json::to_string(&h)?
        } else { // can be here if no exposed port
            "non-service".into()
        };
        let (support, supportlink) = (md.support.clone(), md.support.unwrap().link(&cfg.slack));
        // TODO: org in config
        let circlelink = format!("https://circleci.com/gh/Babylonpartners/{}", mf.name);
        let quaylink = format!("https://{}/?tab=tags", mf.image.clone().unwrap());

        let (team, teamlink) = (md.team.clone(), format!("/raftcat/teams/{}", team_slug(&md.team)));
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
        let s = req.state().render_template("service.tera", ctx);
        Ok(HttpResponse::Ok().content_type("text/html").body(s))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}

fn health(_: &HttpRequest<State>) -> HttpResponse {
    HttpResponse::Ok().json("healthy")
}

fn get_config(req: &HttpRequest<State>) -> Result<HttpResponse> {
    let cfg = req.state().get_config()?;
    Ok(HttpResponse::Ok().json(cfg))
}

#[derive(Serialize)]
struct SimpleManifest {
    name: String,
    team: String,
}

fn index(req: &HttpRequest<State>) -> Result<HttpResponse> {
    let mut ctx = tera::Context::new();
    let mfs = req.state().get_manifests()?;
    let data = mfs.into_iter().map(|(k, m)| {
        SimpleManifest {
            name: k,
            team: m.metadata.unwrap().team.to_lowercase(),
        }
    }).collect::<Vec<_>>();
    let data = serde_json::to_string(&data)?;
    ctx.insert("raftcat", env!("CARGO_PKG_VERSION"));
    ctx.insert("manifests", &data);
    let s = req.state().render_template("index.tera", ctx);
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}


fn main() -> Result<()> {
    sentry::integrations::panic::register_panic_handler();
    let dsn = env::var("SENTRY_DSN").expect("Sentry DSN required");
    let _guard = sentry::init(dsn); // must keep _guard in scope

    env::set_var("RUST_LOG", "actix_web=info,raftcat=info,kubernetes=info");
    if let Ok(level) = env::var("LOG_LEVEL") {
        if level.to_lowercase() == "debug" {
            env::set_var("RUST_LOG", "actix_web=info,raftcat=debug");
        }
    }
    //env::set_var("RUST_BACKTRACE", "full"); // <- don't! this spams logz.io, rely on sentry!
    env_logger::init();

    // TODO: fix so that this path isn't checked at all
    env::set_var("VAULT_TOKEN", "INVALID"); // needed because it happens super early..

    // Load the config: local kube config prioritised first for local development
    // NB: Only supports a config with client certs locally (e.g. kops setup)
    let cfg = match env::var("HOME").expect("have HOME dir").as_ref() {
        "/root" => kubernetes::config::incluster_config(),
        _ => kubernetes::config::load_kube_config(),
    }.expect("Failed to load kube config");
    let shared_state = state::init(cfg).unwrap(); // crash if init fails

    info!("Creating http server");
    let sys = actix::System::new("raftcat");
    server::new(move || {
        App::with_state(shared_state.clone())
            .middleware(middleware::Logger::default()
                .exclude("/raftcat/health")
                .exclude("/health")
                .exclude("/favicon.ico")
                .exclude("/raftcat/static/images/*.png")
            )
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
            .resource("/health", |r| r.method(Method::GET).f(health)) // redundancy
            .resource("/raftcat/", |r| r.method(Method::GET).f(index))
        })
        .bind("0.0.0.0:8080").expect("Can not bind to 0.0.0.0:8080")
        .shutdown_timeout(0)    // <- Set shutdown timeout to 0 seconds (default 60s)
        .start();

    info!("Starting listening on 0.0.0.0:8080");
    let _ = sys.run();
    std::process::exit(0); // SIGTERM ends up here eventually
}
