use std::path::Path;

use super::traits::Verify;
use super::Result;

/// What sensitive data is managed and how
///
/// See https://engineering.ops.babylontech.co.uk/docs/principles-security/
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct DataHandling {
    /// Where and how data is stored
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stores: Vec<DataStore>,
    /// Where the data was retrieved from
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub processes: Vec<DataProcess>,
}

/// Data storage information and encryption information
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct DataStore {
    /// Storage type (one of "MySQL", "DynamoDB", "S3", "File")
    pub backend: String,
    /// Service stores PII
    #[serde(default)]
    pub pii: bool,
    /// Service stores SPII
    #[serde(default)]
    pub spii: bool,
    /// Encryption is in use at the storage side
    ///
    /// If either pii or spii is true, then this must be true
    #[serde(default)]
    pub encrypted: bool,

    /// Fields stored in this backend
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<DataField>,
}

/// Canonical names for data fields
#[derive(Serialize, Deserialize, Clone)]
pub enum DataFieldType {
    Address,
    EmailAddress,
}


/// Data storage information and encryption information
#[derive(Serialize, Deserialize, Clone,)]
pub struct DataField {
    /// Canonical name of the data field
    pub name: DataFieldType,
    /// Cipher used to encrypt if used
    pub cipher: Option<String>,

    // Data is encryption strategies TODO: does this live in here?
    // Key rotator if used
    //pub keyRotator: Option<String>,
    // Retention period if any
    //pub retentionPeriod: Option<String>,

}



/// Data storage information and encryption information
#[derive(Serialize, Deserialize, Clone)]
pub struct DataProcess {
    /// Canonical field name
    pub field: DataFieldType,
    /// Service source service for this information
    pub source: String,
}


impl Verify for DataHandling {
    fn verify(&self) -> Result<()> {
        for s in &self.stores {
            // can't block on this yet - so just warn a lot
            if s.pii && !s.encrypted {
                warn!("{} stores PII without encryption", s.backend)
            }
            if s.spii && !s.encrypted {
                warn!("{} stores SPII without encryption", s.backend)
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
