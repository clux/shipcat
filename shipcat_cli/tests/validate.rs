extern crate serde_yaml;
mod common;

use crate::common::setup;

extern crate shipcat;
extern crate shipcat_definitions;

use shipcat_definitions::{Config, ConfigType};
use shipcat::validate::manifest as validate;

#[test]
fn validate_test() {
    setup();
    let (conf, reg) = Config::new(ConfigType::Base, "dev-uk").unwrap();
    let res = validate(vec!["fake-ask".into()], &conf, &reg, true);
    assert!(res.is_ok());
    let res2 = validate(vec!["fake-storage".into(), "fake-ask".into()], &conf, &reg, false);
    assert!(res2.is_ok())
}
