mod common;
use crate::common::setup;
use shipcat_definitions::{Config, ConfigType};
use shipcat::helm::values;

#[test]
fn helm_values() {
    setup();
    let (conf, reg) = Config::new(ConfigType::Base, "dev-uk").unwrap();
    let mf = shipcat_filebacked::load_manifest("fake-ask", &conf, &reg).unwrap().stub(&reg).unwrap();
    if let Err(e) = values(&mf, None) {
        println!("Failed to create helm values for fake-ask");
        print!("{}", e);
        assert!(false);
    }
    // can verify output here matches what we want if we wanted to,
    // but type safety proves 99% of that anyway
}
