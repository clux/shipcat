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

// graphing
extern crate petgraph;

#[macro_use]
extern crate log;

extern crate regex;

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

/// A renderer of `tera` templates (jinja style)
pub mod template;
/// A Hashicorp Vault HTTP client using `reqwest`
pub mod vault;
/// Convenience listers
pub mod list;
/// A post interface to slack using `slack_hook`
pub mod slack;

/// Structs for the manifest
pub mod structs;

mod manifest;
pub use manifest::{validate, Manifest};

/// Templating consumer module
pub mod generate;

/// A small CLI kubernetes interface
pub mod kube;

/// A graph generator for manifests using `petgraph`
pub mod graph;

// Test helpers
#[cfg(test)]
extern crate loggerv;
#[cfg(test)]
mod tests {
    use std::env;
    use loggerv;
    use std::fs;
    use std::path::Path;

    use std::sync::{Once, ONCE_INIT};
    static START: Once = ONCE_INIT;

    /// Set cwd to tests directory to be able to test manifest functionality
    ///
    /// The tests directory provides a couple of fake services for verification
     pub fn setup() {
        START.call_once(|| {
            env::set_var("SHIPCAT_MANIFEST_DIR", env::current_dir().unwrap());
            loggerv::Logger::new()
                .verbosity(1) // TODO: filter tokio/hyper and bump
                .module_path(true)
                .line_numbers(true)
                .init()
                .unwrap();
            let pwd = env::current_dir().unwrap();
            let testdir = fs::canonicalize(Path::new(&pwd).join("tests")).unwrap();
            info!("Initializing tests - using testdir {}", testdir.display());
            assert!(env::set_current_dir(testdir).is_ok());
        });
    }
}
