use std::collections::BTreeMap;
use config::{Config};

use super::{Manifest};
use states::{ManifestType};

/// Basic CRD wrapper struct
#[derive(Serialize, Deserialize, Clone)]
pub struct Crd<T>
{
    pub apiVersion: String,
    pub kind: String,
    pub metadata: Metadata,
    pub spec: T,
}
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Metadata {
    pub name: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, String>,
    // TODO: generation / resourceVersion later
}

impl From<Manifest> for Crd<Manifest> {
    fn from(mf: Manifest) -> Crd<Manifest> {
        // we assume the manifest has all it needs to fill in the pieces
        // but no secrets!
        assert_eq!(mf.kind, ManifestType::Base);
        Crd {
            apiVersion: "babylontech.co.uk/v1".into(),
            kind: "ShipcatManifest".into(),
            metadata: Metadata {
                name: format!("{}", mf.name),
                ..Metadata::default()
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
        let rname = rgs[0].to_owned();
        Crd {
            apiVersion: "babylontech.co.uk/v1".into(),
            kind: "ShipcatConfig".into(),
            metadata: Metadata {
                name: rname, ..Metadata::default()
            },
            spec: conf,
        }
    }
}


/// Basic CRD List wrapper struct
#[derive(Deserialize, Serialize)]
pub struct CrdList<T> {
    pub apiVersion: String,
    pub kind: String,
    //pub metadata: Metadata,
    pub items: Vec<Crd<T>>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum CrdEventType {
    Added,
    Modified,
    Deleted,
}

/// Basic CRD Watch wrapper struct
#[derive(Deserialize, Serialize)]
pub struct CrdEvent<T> {
    #[serde(rename = "type")]
    pub kind: CrdEventType,
    pub object: Crd<T>,
}
