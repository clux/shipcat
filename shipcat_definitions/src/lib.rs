#![recursion_limit = "1024"]
#![allow(renamed_and_removed_lints)]
#![allow(non_snake_case)]

//extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate serde_json;
extern crate serde;

#[macro_use]
extern crate tera;
#[cfg(feature = "filesystem")]
extern crate walkdir;

#[cfg(feature = "filesystem")]
extern crate dirs;

#[macro_use]
extern crate log;

extern crate reqwest;

extern crate regex;

extern crate semver;
extern crate base64;

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
        Reqw(reqwest::UrlError);
        Reqe(reqwest::Error);
        Time(::std::time::SystemTimeError);
    }
    errors {
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
        InvalidTemplate(svc: String) {
            description("invalid template")
            display("service '{}' has invalid templates", svc)
        }
        InvalidManifest(svc: String) {
            description("manifest does not validate")
            display("manifest for {} does not validate", &svc)
        }
        InvalidSecretForm(key: String) {
            description("secret is of incorrect form")
            display("secret '{}' not have the 'value' key", &key)
        }
        SecretNotAccessible(key: String) {
            description("secret could not be reached or accessed")
            display("secret '{}'", &key)
        }
    }
}

/// Master config for manifests repositories
pub mod config;
pub use config::{Config, Region, Team, VaultConfig, VersionScheme, ManifestDefaults};


/// Structs for the manifest
pub mod structs;

pub mod manifest;
pub use manifest::Manifest;

/// Crd wrappers
mod crds;
pub use crds::{Crd, CrdList};

/// Internal classifications and states
mod states;
pub use states::{ConfigType};

/// File backing
#[cfg(feature = "filesystem")]
mod filebacked;

// Merge behaviour for manifests
mod merge;

/// Computational helpers
pub mod math;


/// A renderer of `tera` templates (jinja style)
///
/// Used for small app configs that are inlined in the completed manifests.
pub mod template;

//pub mod product;
//pub use product::Product;

/// A Hashicorp Vault HTTP client using `reqwest`
pub mod vault;
pub use vault::Vault;
