use std::path::Path;

use super::traits::Verify;
use super::{Result, Config};

/// What sensitive data is managed and how
///
/// See https://engineering.ops.babylontech.co.uk/docs/principles-security/
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
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

/// Data storage information and encryption information
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct DataStore {
    /// Storage type (one of "MySQL", "DynamoDB", "S3", "File")
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

/// Canonical names for data fields
///
/// This is to indicate the canonical data type, not the actual field names.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DataFieldType {
    FullName,
    HomeAddress,
    DateOfBirth,
    EmailAddress,
    BabylonUserId,
    FacebookUserId,
    FacebookAuthToken,
    PaymentDetails,
    PrescriptionHistory,
    AppointmentHistory,
    TransactionHistory,
    ReferralHistory,
    ChatHistory,
    FutureAppointments,
    ConsultationNotes,
    ConsultationVideoRecordings,
    ConsultationAudioRecordings,
    ChatbotRawUserString,
    DeviceHistory,
    PhoneNumber,
    PgmFlowOutcomes,
    CheckbaseFlowOutcomes,
    /// Internal babylon health check metric
    HealthCheck,
}

// https://engineering.ops.babylontech.co.uk/docs/principles-security/#what-is-sensitive-personally-identifiable-information
impl DataFieldType {
    fn is_pii(&self) -> bool {
        // Matching by exclusion by default
        match self {
            &DataFieldType::HealthCheck => false,
            _ => true
        }
    }
    fn is_spii(&self) -> bool {
        match self {
            &DataFieldType::FullName => false,
            &DataFieldType::HomeAddress => false,
            &DataFieldType::DateOfBirth => false,
            // Otherwise fall back to the weaker PII
            // because: not PII implies not SPII
            // slightly more sensible default than just `true`
            _ => self.is_pii()
        }
    }
}


/// Data storage information and encryption information
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct DataField {
    /// Canonical name of the data field
    pub name: DataFieldType,

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
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct DataProcess {
    /// Canonical field name
    pub field: DataFieldType,
    /// Service source service for this information
    pub source: String,
}

impl Verify for DataHandling {
    fn verify(&self, _: &Config) -> Result<()> {
        for s in &self.stores {
            for f in &s.fields {
                let enc = f.encrypted.unwrap(); // filled by implicits
                // can't block on this yet - so just warn a lot
                if f.name.is_spii() && !enc {
                    warn!("{} stores SPII ({:?}) without encryption", s.backend, f.name)
                }
                // weaker warning
                else if f.name.is_pii() && !enc {
                    warn!("{} stores PII ({:?}) without encryption", s.backend, f.name)
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
