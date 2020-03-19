#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "filesystem", serde(deny_unknown_fields))]
pub struct VaultOpts {
    /// If Vault name differs from service name
    pub name: String,
}
