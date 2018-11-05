extern crate serde_yaml;
mod common;

use common::setup;

extern crate shipcat;
extern crate shipcat_definitions;

use shipcat_definitions::{Config};
use shipcat::validate::manifest as validate;

#[test]
fn validate_test() {
    setup();
    let conf = Config::read().unwrap();
    let (_, region) = conf.get_region("dev-uk").unwrap();
    let res = validate(vec!["fake-ask".into()], &conf, &region, true);
    assert!(res.is_ok());
    let res2 = validate(vec!["fake-storage".into(), "fake-ask".into()], &conf, &region, false);
    assert!(res2.is_ok())
}
