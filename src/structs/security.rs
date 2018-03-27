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

    /// Fields stored in this backend
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<DataField>,
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
pub struct DataField {
    /// Canonical name of the data field
    pub name: DataFieldType,
    /// Encryption is in use at the storage side
    ///
    /// If either pii or spii is true, then this must be true
    #[serde(default)]
    pub encrypted: bool,
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
pub struct DataProcess {
    /// Canonical field name
    pub field: DataFieldType,
    /// Service source service for this information
    pub source: String,
}


impl Verify for DataHandling {
    fn verify(&self) -> Result<()> {
        for s in &self.stores {
            for f in &s.fields {
                // can't block on this yet - so just warn a lot
                if f.name.is_pii() && !f.encrypted {
                    warn!("{} stores PII ({:?}) without encryption", s.backend, f.name)
                }
                if f.name.is_spii() && !f.encrypted {
                    warn!("{} stores SPII without encryption", s.backend)
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
