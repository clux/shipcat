mod common;
use crate::common::setup;
use shipcat::{helm, Result};
use shipcat_definitions::{Config, ConfigState};

#[test]
#[ignore] // This test requires helm cli - not on circle
fn helm_template() -> Result<()> {
    setup();
    let (conf, reg) = Config::new(ConfigState::Base, "dev-uk")?;
    let mf = shipcat_filebacked::load_manifest("fake-storage", &conf, &reg)?.stub(&reg)?;

    let res = helm::template(&mf, None)?;

    // verify we have deferred to helm for templating
    assert!(res.contains("image: \"quay.io/babylonhealth/fake-ask:1.6.0\""));
    Ok(())
}
