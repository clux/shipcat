use super::Result;
use regex::Regex;
use std::path::Path;

/// What sensitive data is managed and how
///
/// See https://engineering.ops.babylontech.co.uk/docs/principles-security/
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[cfg_attr(feature = "filesystem", serde(deny_unknown_fields))]
pub struct DataHandling {
    /// Where and how data is stored
    #[serde(default)]
    pub stores: Vec<DataStore>,
    /// Where the data was retrieved from
    #[serde(default)]
    pub processes: Vec<DataProcess>,
}

impl DataHandling {
    pub fn implicits(&mut self) {
        for s in &mut self.stores {
            s.implicits();
        }
    }
}

/// Possible levels of information classification of the data stored in the data store.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum InformationClassification {
    StrictlyConfidential,
    ConfidentialPatientData,
    CommercialConfidential,
    ProtectedInternal,
    Public,
}

impl Default for InformationClassification {
    fn default() -> Self {
        InformationClassification::ConfidentialPatientData
    }
}

/// Data storage information and encryption information
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[cfg_attr(feature = "filesystem", serde(deny_unknown_fields))]
pub struct DataStore {
    /// Storage type (one of "MySQL", "DynamoDB", "S3", "File", "Kafka")
    pub backend: String,

    /// Fields stored in this backend
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<DataField>,

    /// Encryption is in use at the storage side
    ///
    /// If either pii or spii is true, then this must be true
    #[serde(default)]
    pub encrypted: Option<bool>,
    /// Cipher used to encrypt if used
    pub cipher: Option<String>,
    // Data is encryption strategies TODO: does this live in here?
    /// Key rotator if used TODO: format?
    pub keyRotator: Option<String>,
    /// Retention period if any TODO: format? humantime?
    pub retentionPeriod: Option<String>,

    /// The information classification of the data stored in the data store.
    pub informationClassification: Option<InformationClassification>,
}

impl DataStore {
    // Cascase DataStore level encryption params to the fields if none set there
    pub fn implicits(&mut self) {
        for f in &mut self.fields {
            // If field values are not set, set them to the DataStores values
            // if neither are set, clarify missing encryption value => no encryption
            if f.encrypted.is_none() {
                if let Some(e) = self.encrypted {
                    f.encrypted = Some(e);
                } else {
                    f.encrypted = Some(false);
                }
            }
            // For the Option<String> types, we override only in the one clean case:
            // outer value is set, but not the inner.
            //
            // If however, inner is set but not outer, or both set. Nothing to do.
            // If neither is set, everything is left as None types.
            if f.cipher.is_none() && self.cipher.is_some() {
                f.cipher = self.cipher.clone();
            }
            if f.keyRotator.is_none() && self.keyRotator.is_some() {
                f.keyRotator = self.keyRotator.clone();
            }
            if f.retentionPeriod.is_none() && self.retentionPeriod.is_some() {
                f.retentionPeriod = self.retentionPeriod.clone();
            }
        }
    }
}

/// Data storage information and encryption information
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "filesystem", serde(deny_unknown_fields))]
pub struct DataField {
    /// Canonical name of the data field
    pub name: String,

    // same encryption params as in DataStore
    // TODO: #[serde(flatten)] when we can
    /// Encryption is in use at the storage side
    ///
    /// If either pii or spii is true, then this must be true
    pub encrypted: Option<bool>,
    /// Cipher used to encrypt if used
    pub cipher: Option<String>,
    // Data is encryption strategies TODO: does this live in here?
    // Key rotator if used
    pub keyRotator: Option<String>,
    // Retention period if any
    pub retentionPeriod: Option<String>,
}

/// Data storage information and encryption information
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "filesystem", serde(deny_unknown_fields))]
pub struct DataProcess {
    /// Canonical field name
    pub field: String,
    /// Service source service for this information
    pub source: String,
}

impl DataHandling {
    pub fn verify(&self) -> Result<()> {
        // field names must be PascalCase
        let re = Regex::new(r"^[A-Z][[:alpha:]\d]+$").unwrap();
        for s in &self.stores {
            for f in &s.fields {
                if !re.is_match(&f.name) {
                    bail!(
                        "The field {} is not valid PascalCase, or starts with a number",
                        f.name
                    );
                }
            }
        }
        for p in &self.processes {
            let sourcepth = Path::new(".").join("services").join(&p.source);
            if !sourcepth.is_dir() {
                bail!("Service {} does not exist in services/", p.source);
            }
        }
        Ok(())
    }
}
