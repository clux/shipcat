extern crate semver;
extern crate shipcat;
extern crate shipcat_definitions;

use self::semver::Version;
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::{Once, ONCE_INIT};

static START: Once = ONCE_INIT;

macro_rules! btree_set {
    ( $( $x:expr ),* ) => {
        {
            let mut set = BTreeSet::new();
            $(
                set.insert($x);
            )*
            set
        }
    };
}

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

use shipcat_definitions::{Config, Manifest}; // Product
use shipcat_definitions::ConfigType;

#[test]
fn config_test() {
    setup();
    assert!(Config::read().is_ok());
    assert!(Config::new(ConfigType::Base, "dev-uk").is_ok());
    let fullcfg = Config::new(ConfigType::Completed, "dev-uk");
    assert!(fullcfg.is_ok());
    assert!(Config::new(ConfigType::File, "dev-uk").is_err());
    let (conf, _region) = fullcfg.unwrap();
    assert!(conf.print().is_ok());
}

#[test]
fn config_defaults_test() {
    setup();
    let (conf, reg) = Config::new(ConfigType::Base, "dev-uk").unwrap();

    // -- Slack channels --

    // fake-ask gets default for 'devops'
    let mfdefault = Manifest::base("fake-ask", &conf, &reg).unwrap().complete(&reg).unwrap();
    let metadata = mfdefault.metadata.unwrap();
    assert_eq!(*metadata.support.unwrap(), "#devops-support");
    assert_eq!(*metadata.notifications.unwrap(), "#devops-notifications");

    // fake-storage overrides for 'someteam'
    let mfoverride = Manifest::base("fake-storage", &conf, &reg).unwrap().complete(&reg).unwrap();
    let metadata = mfoverride.metadata.unwrap();
    assert_eq!(*metadata.support.unwrap(), "#dev-platform-override");
    assert_eq!(*metadata.notifications.unwrap(), "#dev-platform-notif-override");
}

/*#[test]
fn product_test() {
    setup();
    let conf = Config::read().unwrap();
    let p = Product::completed("triage", &conf, "uk").unwrap();
    let res = p.verify(&conf);
    assert!(res.is_ok(), "verified product");
}
*/

use shipcat::get;
#[test]
fn getters() {
    setup();
    let (conf, reg) = Config::new(ConfigType::Base, "dev-uk").unwrap();
    let vers = get::versions(&conf, &reg).unwrap();
    assert_eq!(vers.len(), 1); // only one of the services has a version
    assert_eq!(vers["fake-ask"], Version::new(1, 6, 0));

    let imgs = get::images(&conf, &reg).unwrap();
    assert_eq!(imgs.len(), 2); // every service gets an image
    assert_eq!(imgs["fake-ask"], "quay.io/babylonhealth/fake-ask");
    assert_eq!(imgs["fake-storage"], "nginx");
}

#[test]
fn clusterinfo() {
    setup();
    // clusterinfo must be correct in its resolution!
    // test all cases
    // NB: needs a base config to be able to verify region/cluster constraints
    let conf = Config::read().unwrap();

    assert!(get::clusterinfo(&conf, "preproduk-blue").is_ok());
    assert!(get::clusterinfo(&conf, "preproduk-green").is_ok());
    assert!(get::clusterinfo(&conf, "preprod-uk").is_err()); // ambiguous

    let blue = get::clusterinfo(&conf, "preproduk-blue").unwrap();
    assert_eq!(blue.region, "preprod-uk"); // correctly resolved

    assert!(get::clusterinfo(&conf, "dev-global").is_ok());
    let devglob = get::clusterinfo(&conf, "dev-global").unwrap();
    assert_eq!(devglob.region, "dev-global");
    assert_eq!(devglob.cluster, "kops-global")
}

#[test]
fn get_codeowners() {
    setup();
    let conf = Config::read().unwrap();
    let cos = get::codeowners(&conf).unwrap();

    assert_eq!(cos.len(), 1); // services without owners get no listing
    assert_eq!(cos[0], "services/fake-ask/* @clux");
}

#[test]
fn manifest_test() {
    setup();
    let (conf, reg) = Config::new(ConfigType::Base, "dev-uk").unwrap();
    let mfread = Manifest::base("fake-storage", &conf, &reg);
    assert!(mfread.is_ok());
    let mfbase = mfread.unwrap();
    let mfres = mfbase.complete(&reg);
    assert!(mfres.is_ok());
    let mf = mfres.unwrap();

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
    let (conf, reg) = Config::new(ConfigType::Base, "dev-uk").unwrap();
    let mf = Manifest::base("fake-ask", &conf, &reg).unwrap().complete(&reg).unwrap();

    // verify templating
    let env = mf.env.plain;
    assert_eq!(env["CORE_URL"], "https://woot.com/somesvc".to_string());
    // check values from Config - one plain, one as_secret
    assert_eq!(env["CLIENT_ID"], "FAKEASKID".to_string());
    assert!(env.get("CLIENT_SECRET").is_none()); // moved to secret

    assert_eq!(
        mf.env.secrets,
        btree_set!["CLIENT_SECRET".to_string(), "FAKE_SECRET".to_string()]
    );

    // verify sidecar templating
    let redis = &mf.sidecars[0];
    assert_eq!(redis.env.plain["STATIC_VALUE"], "static".to_string());
    assert_eq!(
        redis.env.plain["CORE_URL"],
        "https://woot.com/somesvc".to_string()
    );
    assert_eq!(
        redis.env.secrets,
        btree_set!["FAKE_NUMBER".to_string(), "FAKE_SECRET".to_string()]
    );

    // verify worker templating
    let w = &mf.workers[0];
    assert_eq!(w.env.plain["URL"], "https://woot.com/worker".to_string());
    assert_eq!(w.env.secrets, BTreeSet::new());

    let c = &mf.cronJobs[0];
    assert_eq!(c.env.plain["URL"], "https://woot.com/cronjob".to_string());
    assert_eq!(c.env.secrets, BTreeSet::new());

    // verify secrets
    let sec = mf.secrets;
    assert_eq!(sec["CLIENT_SECRET"], "FAKEASKSECRET".to_string()); // via reg.kong consumers
    assert_eq!(sec["FAKE_SECRET"], "hello".to_string()); // NB: ACTUALLY IN_VAULT
    assert_eq!(sec["FAKE_NUMBER"], "-2".to_string()); // NB: ACTUALLY IN_VAULT

    let configs = mf.configs.clone().unwrap();
    let configini = configs.files[0].clone();
    let cfgtpl = configini.value.unwrap();
    print!("{:?}", cfgtpl);
    assert!(cfgtpl.contains("CORE=https://woot.com/somesvc"));
    assert!(cfgtpl.contains("CLIENT_ID"));
    assert!(cfgtpl.contains("CLIENT_ID=FAKEASKID"));
}
