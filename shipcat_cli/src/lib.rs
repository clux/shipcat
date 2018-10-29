#![recursion_limit = "1024"]
#![allow(renamed_and_removed_lints)]
#![allow(non_snake_case)]

#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate serde;

// templating
//#[macro_use]
//extern crate tera;
extern crate walkdir;

// vault api
extern crate reqwest;
#[macro_use]
extern crate serde_json;

extern crate openssl_probe;

// jenkins api
extern crate jenkins_api;
extern crate chrono;


// notifications
extern crate slack_hook;

// graphing
extern crate petgraph;

#[macro_use]
extern crate log;

extern crate regex;

extern crate semver;

extern crate threadpool;

extern crate base64;

extern crate dirs;


#[macro_use]
extern crate error_chain;
error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }
    links {}
    foreign_links {
        Fmt(::std::fmt::Error);
        Io(::std::io::Error) #[cfg(unix)];
        Float(::std::num::ParseFloatError);
        Int(::std::num::ParseIntError);
        Mani(shipcat_definitions::Error);
        //Tmpl(tera::Error);
        SerdeY(serde_yaml::Error);
        SerdeJ(serde_json::Error);
        Slack(slack_hook::Error);
        Reqw(reqwest::UrlError);
        Reqe(reqwest::Error);
        Time(::std::time::SystemTimeError);
    }
    errors {
        MissingJenkinsJob(job: String) {
            description("Jenkins job could not be fetched")
            display("Failed to get jenkins job {}", job)
        }
        JenkinsFailure {
            description("Jenkins client configuration failure")
            display("Failed to create jenkins client")
        }
        MissingSlackUrl {
            description("SLACK_SHIPCAT_HOOK_URL not specified")
            display("SLACK_SHIPCAT_HOOK_URL not specified")
        }
        MissingSlackChannel {
            description("SLACK_SHIPCAT_CHANNEL not specified")
            display("SLACK_SHIPCAT_CHANNEL not specified")
        }
        MissingGrafanaUrl {
            description("GRAFANA_SHIPCAT_HOOK_URL not specified")
            display("GRAFANA_SHIPCAT_HOOK_URL not specified")
        }
        MissingGrafanaToken {
            description("GRAFANA_SHIPCAT_TOKEN not specified")
            display("GRAFANA_SHIPCAT_TOKEN not specified")
        }
        MissingJenkinsUrl {
            description("JENKINS_API_URL not specified")
            display("JENKINS_API_URL not specified")
        }
        MissingJenkinsUser {
            description("JENKINS_API_USER not specified")
            display("JENKINS_API_USER not specified")
        }
        UnexpectedHttpStatus(status: reqwest::StatusCode) {
            description("unexpected HTTP status")
            display("unexpected HTTP status: {}", &status)
        }
        NoHomeDirectory {
            description("can't find home directory")
            display("can't find home directory")
        }
        Url(url: reqwest::Url) {
            description("could not access URL")
            display("could not access URL '{}'", &url)
        }
        InvalidSecretForm(key: String) {
            description("secret is of incorrect form")
            display("secret '{}' not have the 'value' key", &key)
        }
        SecretNotAccessible(key: String) {
            description("secret could not be reached or accessed")
            display("secret '{}'", &key)
        }
        MissingRollingVersion(svc: String) {
            description("missing version for install")
            display("{} has no version in manifest and is not installed yes", &svc)
        }
        ManifestFailure(key: String) {
            description("Manifest key not propagated correctly internally")
            display("manifest key {} was not propagated internally - bug!", &key)
        }
        ManifestVerifyFailure(svc: String) {
            description("manifest does not validate")
            display("manifest for {} does not validate", &svc)
        }
        HelmUpgradeFailure(svc: String) {
            description("Helm upgrade call failed")
            display("Helm upgrade of {} failed", &svc)
        }
        UpgradeTimeout(svc: String, secs: u32) {
            description("upgrade timed out")
            display("{} upgrade timed out waiting {}s for deployment(s) to come online", &svc, secs)
        }
        SlackSendFailure(hook: String) {
            description("slack message send failed")
            display("Failed to send the slack message to '{}' ", &hook)
        }
    }
}

extern crate shipcat_definitions;
pub use shipcat_definitions::{Manifest, Product};
pub use shipcat_definitions::structs;
pub use shipcat_definitions::config::{self, Config, VersionScheme};


/// Convenience listers
pub mod list;
/// A post interface to slack using `slack_hook`
pub mod slack;
/// A REST interface to grafana using `reqwest`
pub mod grafana;
/// Cluster level operations
pub mod cluster;

/// Validation methods of manifests post merge
pub mod validate;

/// gdpr lister
pub mod gdpr;

/// A small CLI kubernetes interface
pub mod kube;

/// A small CLI helm interface
pub mod helm;

/// A small CLI kong config generator interface
pub mod kong;

/// A graph generator for manifests using `petgraph`
pub mod graph;

/// A jenkins helper interface using `jenkinsapi`
pub mod jenkins;

/// Various simple reducers
pub mod get;

/// Smart initialiser with safety
///
/// Tricks the library into reading from your manifest location.
pub fn init() -> Result<()> {
    use std::env;
    use std::path::Path;
    openssl_probe::init_ssl_cert_env_vars(); // prerequisite for https clients

    // Allow shipcat calls to work from anywhere if we know where manifests are
    if let Ok(mdir) = env::var("SHIPCAT_MANIFEST_DIR") {
        let pth = Path::new(&mdir);
        if !pth.is_dir() {
            bail!("SHIPCAT_MANIFEST_DIR must exist");
        }
        env::set_current_dir(pth)?;
    }

    Ok(())
}
