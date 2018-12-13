#![warn(rust_2018_idioms)]

mod common;

use std::env;

use shipcat;
use shipcat_definitions;

use crate::shipcat::webhooks;
use crate::shipcat_definitions::{Config, ConfigType};

#[test]
// #[serial] not available still :( so have to concatenate two tests messing with evars
fn webhooks_ensure_requirements() {
    common::setup();

    let (_conf, reg) = Config::new(ConfigType::Completed, "dev-uk").unwrap();

    env::set_var("SHIPCAT_AUDIT_REVISION", "egrevision");
    env::set_var("SHIPCAT_AUDIT_CONTEXT_ID", "egcontextid");
    assert!(webhooks::ensure_requirements(&reg).is_ok());

    env::remove_var("SHIPCAT_AUDIT_REVISION");
    assert!(webhooks::ensure_requirements(&reg).is_err());
}
