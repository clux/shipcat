#![warn(rust_2018_idioms)]

#[macro_use] extern crate serde_derive;
#[macro_use] extern crate log;
#[macro_use] extern crate failure;

pub use failure::Error;
pub type Result<T> = std::result::Result<T, Error>;

pub use shipcat_definitions::{Manifest, Config, Cluster, Region, Team};

/// A small CLI kubernetes interface
pub mod kube;
pub use crate::kube::{ManifestMap, ManifestCache};

/// Integrations with external solutions like sentry/newrelic etc
pub mod integrations;

/// State machinery for actix
pub mod state;
pub use state::State;
