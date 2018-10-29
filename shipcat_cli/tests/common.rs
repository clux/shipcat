extern crate semver;
extern crate shipcat;
extern crate shipcat_definitions;

use self::semver::Version;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::{Once, ONCE_INIT};

static START: Once = ONCE_INIT;

/// Set cwd to tests directory to be able to test manifest functionality
///
/// The tests directory provides a couple of fake services for verification
pub fn setup() {
    START.call_once(|| {
        let pwd = env::current_dir().unwrap();
        let pth = fs::canonicalize(Path::new(&pwd).join("..").join("tests")).unwrap();
        env::set_var("SHIPCAT_MANIFEST_DIR", pth.clone());
        //loggerv::Logger::new()
        //    .verbosity(1) // TODO: filter tokio/hyper and bump
        //    .module_path(true)
        //    .line_numbers(true)
        //    .init()
        //    .unwrap();
        // TODO: stop creating multiple reqwest clients in tests, might not be safe
        println!("Initializing tests - using testdir {}", pth.display());
        assert!(env::set_current_dir(pth).is_ok());
    });
}

use shipcat_definitions::{Config, Product, Manifest};

#[test]
fn product_test() {
    setup();
    let conf = Config::read().unwrap();
    let p = Product::completed("triage", &conf, "uk").unwrap();
    let res = p.verify(&conf);
    assert!(res.is_ok(), "verified product");
}

#[test]
fn get_versions() {
    setup();
    let conf = Config::read().unwrap();
    let vers = Manifest::get_versions(&conf, "dev-uk").unwrap();

    assert_eq!(vers.len(), 1); // only one of the services has a version
    assert_eq!(vers["fake-ask"], Version::new(1, 6, 0));
}

#[test]
fn get_images() {
    setup();
    let conf = Config::read().unwrap();
    let vers = Manifest::get_images(&conf, "dev-uk").unwrap();

    assert_eq!(vers.len(), 2); // every service gets an image
    assert_eq!(vers["fake-ask"], "quay.io/babylonhealth/fake-ask");
    assert_eq!(vers["fake-storage"], "nginx");
}

#[test]
fn get_codeowners() {
    setup();
    let conf = Config::read().unwrap();
    let cos = Manifest::get_codeowners(&conf, "dev-uk").unwrap();

    assert_eq!(cos.len(), 1); // services without owners get no listing
    assert_eq!(cos[0], "services/fake-ask/* @clux");
}



use shipcat_definitions::structs::HealthCheck;

#[test]
fn wait_time_check() {
    setup();
    // DEFAULT SETUP: no values == defaults => 180s helm wait
    let mut mf = Manifest::default();
    mf.imageSize = Some(512);
    mf.health = Some(HealthCheck {
        uri: "/".into(),
        wait: 30,
        ..Default::default()
    });
    mf.replicaCount = Some(2);
    let wait = mf.estimate_wait_time();
    assert_eq!(wait, (30+60)*2);

    // setup with large image and short boot time:
    mf.imageSize = Some(4096);
    mf.health = Some(HealthCheck {
        uri: "/".into(),
        wait: 20,
        ..Default::default()
    });
    let wait2 = mf.estimate_wait_time();
    assert_eq!(wait2, (20+480)*2);
}

#[test]
fn manifest_test() {
    setup();
    let conf = Config::read().unwrap();
    let mf = Manifest::completed("fake-storage", &conf, "dev-uk".into()).unwrap();
    // verify datahandling implicits
    let dh = mf.dataHandling.unwrap();
    let s3 = dh.stores[0].clone();
    assert!(s3.encrypted.unwrap());
    assert_eq!(s3.fields[0].encrypted.unwrap(), false); // overridden
    assert_eq!(s3.fields[1].encrypted.unwrap(), true); // cascaded
    assert_eq!(s3.fields[0].keyRotator, None); // not set either place
    assert_eq!(s3.fields[1].keyRotator, Some("2w".into())); // field value
}

#[test]
fn templating_test() {
    setup();
    let conf = Config::read().unwrap();
    let mf = Manifest::completed("fake-ask", &conf, "dev-uk".into()).unwrap();

    // verify templating
    let env = mf.env;
    assert_eq!(env["CORE_URL"], "https://woot.com/somesvc".to_string());
    // check values from Config - one plain, one as_secret
    assert_eq!(env["CLIENT_ID"], "FAKEASKID".to_string());
    assert!(env.get("CLIENT_SECRET").is_none()); // moved to secret
    let sec = mf.secrets;
    assert_eq!(sec["CLIENT_SECRET"], "FAKEASKSECRET".to_string()); // via reg.kong consumers
    assert_eq!(sec["FAKE_SECRET"], "hello".to_string()); // NB: ACTUALLY IN_VAULT

    let configs = mf.configs.clone().unwrap();
    let configini = configs.files[0].clone();
    let cfgtpl = configini.value.unwrap();
    print!("{:?}", cfgtpl);
    assert!(cfgtpl.contains("CORE=https://woot.com/somesvc"));
    assert!(cfgtpl.contains("CLIENT_ID"));
    assert!(cfgtpl.contains("CLIENT_ID=FAKEASKID"));
}
