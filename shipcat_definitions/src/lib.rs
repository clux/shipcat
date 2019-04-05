#![recursion_limit = "1024"]
#![allow(renamed_and_removed_lints)]
#![allow(non_snake_case)]
#![warn(rust_2018_idioms)]

#[macro_use] extern crate serde_derive;
#[macro_use] extern crate log;

/// The backing for manifests must come from the filesystem or the CRD
/// This assert enforce that users of this library choses a feature.
static_assertions::assert_cfg!(all(not(all(feature = "filesystem", feature = "crd")),
                any(    feature = "filesystem", feature = "crd")),
"shipcat definitions library behaves differently depending on compile time features:\n\n\
Please `cargo build -p shipcat` or `cargo build -p raftcat` to force a backend choice, \
or build from shipcat_definitions/ with --features to build the library directly.\n");

#[macro_use] extern crate error_chain; // bail and error_chain macro
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

/// Config with regional data
pub mod region;
pub use crate::region::{Region, VaultConfig, VersionScheme, KongConfig, Environment};
/// Master config with cross-region data
pub mod config;
pub use crate::config::{Config, Cluster, Team, ManifestDefaults};


/// Structs for the manifest
pub mod structs;

pub mod manifest;
pub use crate::manifest::Manifest;

pub mod base;
pub use crate::base::BaseManifest;

/// Crd wrappers
mod crds;
pub use crate::crds::{Crd, CrdList, CrdEvent, CrdEventType, gen_all_crds};

/// Internal classifications and states
mod states;
pub use crate::states::{ConfigType};

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
pub use crate::vault::Vault;

pub mod deserializers;
