#![recursion_limit = "1024"]

#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;

// templating
#[macro_use]
extern crate tera;
extern crate walkdir;

// vault api
extern crate reqwest;
extern crate serde_json;
#[macro_use]
extern crate hyper;

// notifications
extern crate slack_hook;

#[macro_use]
extern crate log;

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
        Tmpl(tera::Error);
        SerdeY(serde_yaml::Error);
        SerdeJ(serde_json::Error);
        Slack(slack_hook::Error);
        Reqw(reqwest::UrlError);
    }
    errors {
        MissingSlackUrl {
            description("SLACK_SHIPCAT_HOOK_URL not specified")
            display("SLACK_SHIPCAT_HOOK_URL not specified")
        }
        MissingSlackChannel {
            description("SLACK_SHIPCAT_CHANNEL not specified")
            display("SLACK_SHIPCAT_CHANNEL not specified")
        }
        MissingVaultAddr {
            description("VAULT_ADDR not specified")
            display("VAULT_ADDR not specified")
        }
        MissingVaultToken {
            description("VAULT_TOKEN not specified")
            display("VAULT_TOKEN not specified")
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
        MissingSecret(key: String) {
            description("secret does not have value for specified key")
            display("secret '{}' does not exist", &key)
        }
    }
}

pub mod template;
pub mod vault;
pub mod list;
pub mod slack;

mod manifest;
pub use manifest::{validate, Manifest};

mod kube;
pub use kube::{ship, generate, Deployment};
