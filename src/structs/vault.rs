#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VaultOpts {
    /// If Vault name differs from service name
    pub name: String,
}
