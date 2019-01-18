use std::collections::BTreeMap;
use crate::config::{Config};

use super::{Manifest};
use crate::states::{ManifestType};

/// Basic CRD wrapper struct
#[derive(Serialize, Deserialize, Clone)]
pub struct Crd<T> {
    pub apiVersion: String,
    pub kind: String,
    pub metadata: Metadata,
    pub spec: T,
}
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Metadata {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, String>,
    // TODO: generation?
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub resourceVersion: String,
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
                name: mf.name.clone(),
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
        let rname: String = if rgs.len() == 1 { // config has been filtered
            // thus, can infer the region :-)
            rgs[0].to_owned()
        } else { // non-filtered
            "unionised".to_owned()
        };

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

// Some extra wrappers for kube api

/// Basic CRD List wrapper struct
#[derive(Deserialize, Serialize)]
pub struct CrdList<T> {
    pub apiVersion: String,
    pub kind: String,
    pub metadata: Metadata,
    pub items: Vec<Crd<T>>,
}

/// Types of events returned from watch requests
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum CrdEventType {
    Added,
    Modified,
    Deleted,
}

/// CRD Event wrapper
///
/// This needs to be parsed per line from a kube api watch request.
#[derive(Deserialize, Serialize)]
pub struct CrdEvent<T> {
    #[serde(rename = "type")]
    pub kind: CrdEventType,
    pub object: Crd<T>,
}
