#![warn(rust_2018_idioms)]

mod common;

use std::env;

use shipcat;
use shipcat_definitions;

use crate::shipcat::webhooks;
use crate::shipcat_definitions::{Config, ConfigType};

#[test]
fn webhooks_ensure_requirements() {
    common::setup();

    env::set_var("SHIPCAT_AUDIT_CONTEXT_ID", "egcontextid");
    env::set_var("SHIPCAT_AUDIT_REVISION", "egrevision");

    let (_conf, reg) = Config::new(ConfigType::Completed, "dev-uk").unwrap();

    assert!(webhooks::ensure_requirements(&reg).is_ok());
}
