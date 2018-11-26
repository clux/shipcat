use std::collections::BTreeMap;
use config::{Config};

use super::{Manifest};
use states::{ManifestType};

/// Basic CRD wrapper struct
#[derive(Serialize, Deserialize, Clone)]
pub struct Crd<T> {
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
}

/// Basic CRD Watch wrapper struct
#[derive(Deserialize, Serialize)]
pub struct CrdEvent<T> {
    #[serde(rename = "type")]
    pub kind: CrdEventType,
    pub object: Crd<T>,
}

pub struct CrdEvents<T>(Vec<CrdEvent<T>>);

impl<T> IntoIterator for CrdEvents<T> {
    type Item = <Vec<CrdEvent<T>> as IntoIterator>::Item;
    type IntoIter = <Vec<CrdEvent<T>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

// Kube gives watch events back in an akward non-separated list of structs
use std::fmt;
use std::marker::PhantomData;
use serde::de::{self, Deserialize, Deserializer, Visitor, MapAccess, SeqAccess};


impl<'de, T> Deserialize<'de> for CrdEvents<T>
where
    T: Deserialize<'de>,
{
}


// A Visitor is a type that holds methods that a Deserializer can drive
// depending on what is contained in the input data.
struct CrdEventsVisitor<T> {
    marker: PhantomData<fn() -> CrdEvents<T>>
}

impl<T> CrdEventsVisitor<T> {
    fn new() -> Self {
        CrdEventsVisitor {
            marker: PhantomData
        }
    }
}

// In order to handle commas correctly when deserializing a JSON array or map,
// we need to track whether we are on the first element or past the first
// element.
struct CommaSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    first: bool,
}

impl<'a, 'de> CommaSeparated<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        CommaSeparated {
            de,
            first: true,
        }
    }
}

impl<'de, T> Visitor<'de> for CrdEventsVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = CrdEvents<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a json list with newline separation and no joining brackets")
    }

    // This function gets given the full data
    fn visit_seq<E>(self, A: SeqAccess) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let mut res = Vec::new();
        for v in value.lines() {
            println!("Got line: {}", v);
            //res.push(self.visit_map(v)?);
        }
        Ok(CrdEvents { 0: res } )
    }

   // Deserialization of compound types like sequences and maps happens by
    // passing the visitor an "Access" object that gives it the ability to
    // iterate through the data contained in the sequence.
    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Parse the opening brace of the element.
        if self.next_char()? == '{' {
            // Give the visitor access to each element
            let value = visitor.visit_map(CommaSeparated::new(&mut self))?;
            // Parse the closing brace of the element.
            if self.next_char()? == '}' {
                Ok(value)
            } else {
                Err(Error::ExpectedMapEnd)
            }
        } else {
            Err(Error::ExpectedMap)
        }
    }
}

impl<'de, T> Deserialize<'de> for CrdEvents<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Instantiate our Visitor and ask the Deserializer to drive
        // it over the input data, resulting in an instance of CrdEvents<T>.
        deserializer.deserialize_seq(CrdEventsVisitor::new())
    }
}
