mod common;
use crate::common::setup;
use shipcat_definitions::{Config, ConfigType};
use shipcat::helm;

#[test]
#[ignore] // This test requires helm cli - not on circle
fn helm_template() {
    setup();
    let (conf, reg) = Config::new(ConfigType::Base, "dev-uk").unwrap();
    let mock = true;
    let res = helm::template("fake-ask", &reg, &conf, None, mock, None).unwrap();

    // verify we have deferred to helm for templating
    assert!(res.contains("image: \"quay.io/babylonhealth/fake-ask:1.6.0\""));
}
