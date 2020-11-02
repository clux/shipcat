use super::{config::ShipcatConfig, manifest::ShipcatManifest, Manifest};
use crate::{config::Config, states::ManifestState};

// We are < 1.17 so use v1beta1
use apiexts::CustomResourceDefinition;
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1beta1 as apiexts;

pub fn gen_all_crds() -> Vec<CustomResourceDefinition> {
    let shipcatManifest = ShipcatManifest::crd();
    let shipcatConfig = ShipcatConfig::crd();
    vec![shipcatConfig, shipcatManifest]
}

impl From<Manifest> for ShipcatManifest {
    fn from(mf: Manifest) -> ShipcatManifest {
        // we assume the manifest has all it needs to fill in the pieces
        // but no secrets!
        assert_eq!(mf.state, ManifestState::Base);
        #[allow(clippy::redundant_clone)] // cloning the name is cheaper than cloning the manifest
        ShipcatManifest::new(&mf.name.clone(), mf)
    }
}

impl From<Config> for ShipcatConfig {
    fn from(conf: Config) -> ShipcatConfig {
        let rgs = conf.list_regions();
        assert!(!conf.has_secrets()); // no secrets
        let allRegs = "unionised";
        let rname: String = if rgs.len() == 1 {
            // config has been filtered
            // thus, can infer the region :-)
            assert_ne!(rgs[0], allRegs); // it'd be silly to name a region like that, right?
            rgs[0].to_owned()
        } else {
            // non-filtered
            allRegs.to_owned()
        };
        ShipcatConfig::new(&rname, conf)
    }
}
