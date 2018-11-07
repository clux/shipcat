use config::{Config};

use super::{Manifest};
use states::{ManifestType};

/// Basic CRD wrapper struct
#[derive(Serialize)]
struct Crd<T> {
    pub apiVersion: String,
    pub kind: String,
    pub metadata: Metadata,
    pub spec: T,
}
#[derive(Serialize)]
struct Metadata {
    name: String
}

impl From<Manifest> for Crd<Manifest> {
    fn from(mf: Manifest) -> Crd<Manifest> {
        // we assume the manifest has all it needs to fill in the pieces
        // but no secrets!
        assert_eq!(mf.kind, ManifestType::Base);
        Crd {
            apiVersion: "shipcat.babylontech.co.uk/v1".into(),
            kind: "ShipcatManifest".into(),
            metadata: Metadata {
                name: format!("{}", mf.name),
            },
            spec: mf,
        }
    }
}

impl From<Config> for Crd<Config> {
    fn from(conf: Config) -> Crd<Config> {
        let rgs = conf.list_regions();
        assert!(!conf.has_secrets()); // no secrets
        assert_eq!(rgs.len(), 1); // config must be filtered
        // thus, can infer the region :-)
        let rname = &rgs[0];
        Crd {
            apiVersion: "shipcat.babylontech.co.uk/v1".into(),
            kind: "ShipcatConfig".into(),
            metadata: Metadata {
                name: rname.to_string()
            },
            spec: conf,
        }
    }
}
