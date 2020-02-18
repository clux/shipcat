mod common;
use crate::common::setup;

use shipcat::validate::manifest as validate;
use shipcat_definitions::{Config, ConfigState};

#[tokio::test]
async fn validate_test() {
    setup();
    let (conf, reg) = Config::new(ConfigState::Base, "dev-uk").await.unwrap();
    let res = validate(vec!["fake-ask".into()], &conf, &reg, true).await;
    assert!(res.is_ok());
    let res2 = validate(vec!["fake-storage".into(), "fake-ask".into()], &conf, &reg, false).await;
    assert!(res2.is_ok())
}
