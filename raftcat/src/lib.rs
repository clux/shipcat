#![allow(renamed_and_removed_lints)]

#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate serde_yaml;

extern crate url;
extern crate http;
extern crate kubernetes;
extern crate reqwest;

#[macro_use] extern crate log;
#[macro_use] extern crate failure;

use failure::{Error}; //Fail
pub type Result<T> = std::result::Result<T, Error>;


extern crate shipcat_definitions;
pub use shipcat_definitions::{Manifest, Config, Team};

/// A small CLI kubernetes interface
pub mod kube;
pub use kube::{ManifestMap};

mod integrations;
pub use integrations::sentryapi::{self, SentryMap};
pub use integrations::newrelic::{self, RelicMap};
pub use integrations::version::{self, VersionMap};
