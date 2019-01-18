use std::collections::BTreeMap;
use crate::config::{Config};

use super::{Manifest};
use crate::states::{ManifestType};

const KUBE_API_VERSION: &str = "apiextensions.k8s.io/v1beta1";
const DOMAIN: &str = "babylontech.co.uk";
const VERSION: &str = "v1";
const SHIPCATCONFIG_KIND: &str = "ShipcatConfig";
const SHIPCATMANIFEST_KIND: &str = "ShipcatManifest";

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

/// Literal CRD - eg for creating definitions against kube api
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct CrdSpec {
    pub group: String,
    pub version: String,
    pub scope: String,
    pub names: CrdNames,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additionalPrinterColumns: Option<Vec<CrdAdditionalPrinterColumns>>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct CrdNames {
    pub plural: String,
    pub singular: String,
    pub kind: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct CrdAdditionalPrinterColumns {
    pub name: String,
    #[serde(rename = "type")]
    pub apcType: String,
    pub description: String,
    pub JSONPath: String,
}

pub fn gen_all_crds() -> Vec<CrdSpec> {
    let shipcatConfig = CrdSpec{
        group: DOMAIN.into(),
        version: VERSION.into(),
        scope: "Namespaced".into(),
        names: CrdNames{
            plural: "shipcatconfigs".into(),
            singular: "shipcatconfig".into(),
            kind: SHIPCATCONFIG_KIND.into(),
        },
        ..CrdSpec::default()
    };
    let shipcatManifest = CrdSpec{
        group: DOMAIN.into(),
        version: VERSION.into(),
        scope: "Namespaced".into(),
        names: CrdNames{
            plural: "shipcatmanifests".into(),
            singular: "shipcatmanifest".into(),
            kind: SHIPCATMANIFEST_KIND.into(),
        },
        additionalPrinterColumns: Some(vec![
            CrdAdditionalPrinterColumns{
                name: "Kong".into(),
                apcType: "string".into(),
                description: "The URI where the service is available through kong".into(),
                JSONPath: ".spec.kong.uris".into(),
            }
        ]),
    };
    vec![shipcatConfig, shipcatManifest]
}

impl From<CrdSpec> for Crd<CrdSpec> {
    fn from(cs: CrdSpec) -> Crd<CrdSpec> {
        Crd {
            apiVersion: KUBE_API_VERSION.into(),
            kind: "CustomResourceDefinition".into(),
            metadata: Metadata {
                name: format!("{}.{}", cs.names.plural, DOMAIN),
                ..Metadata::default()
            },
            spec: cs,
        }
    }
}

impl From<Manifest> for Crd<Manifest> {
    fn from(mf: Manifest) -> Crd<Manifest> {
        // we assume the manifest has all it needs to fill in the pieces
        // but no secrets!
        assert_eq!(mf.kind, ManifestType::Base);
        Crd {
            apiVersion: format!("{}/{}", DOMAIN, VERSION),
            kind: SHIPCATMANIFEST_KIND.into(),
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
            apiVersion: format!("{}/{}", DOMAIN, VERSION),
            kind: SHIPCATCONFIG_KIND.into(),
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
