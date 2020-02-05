mod common;
use crate::common::setup;

use shipcat_definitions::{Config, ConfigState};
use shipcat::validate::manifest as validate;

#[test]
fn validate_test() {
    setup();
    let (conf, reg) = Config::new(ConfigState::Base, "dev-uk").unwrap();
    let res = validate(vec!["fake-ask".into()], &conf, &reg, true);
    assert!(res.is_ok());
    let res2 = validate(vec!["fake-storage".into(), "fake-ask".into()], &conf, &reg, false);
    assert!(res2.is_ok())
}
