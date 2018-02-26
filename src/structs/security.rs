use super::traits::Verify;
use super::Result;

/// What sensitive data is managed and how
///
/// See https://engineering.ops.babylontech.co.uk/docs/principles-security/
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct DataHandling {
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
    pub encrypted: bool,

    // Data is encryption strategies
    /// Key rotator if used
    pub keyRotator: Option<String>,
    /// Cipher used to encrypt if used
    pub cipher: Option<String>,
    /// Retention period if any
    pub retentionPeriod: Option<String>,

    // Services that use this data upstream
    // just use normal dependencies?
    //#[serde(default, skip_serializing_if = "Vec::is_empty")]
    //pub accessedBy: Vec<String>,
}

impl Verify for DataHandling {
    fn verify(&self) -> Result<()> {
        // can't block on this yet - so just warn a lot
        if self.pii && !self.encrypted {
            warn!("{} stores PII without encryption", self.backend)
        }
        if self.spii && !self.encrypted {
            warn!("{} stores SPII without encryption", self.backend)
        }
        Ok(())
    }
}
