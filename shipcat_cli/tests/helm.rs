mod common;
use common::setup;

extern crate shipcat;
extern crate shipcat_definitions;

use shipcat_definitions::{Manifest, Backend, Config};
use shipcat::helm::values;

#[test]
fn helm_values() {
    setup();
    let conf = Config::read().unwrap();
    let (_, region) = conf.get_region("dev-uk").unwrap();
    let mf = Manifest::stubbed("fake-ask", &conf, &region).unwrap();
    if let Err(e) = values(&mf, None) {
        println!("Failed to create helm values for fake-ask");
        print!("{}", e);
        assert!(false);
    }
    // can verify output here matches what we want if we wanted to,
    // but type safety proves 99% of that anyway
}
