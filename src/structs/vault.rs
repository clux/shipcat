#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct VaultOpts {
    /// If Vault name differs from service name
    pub name: String,
    /// If region exists from normal region
    pub region: Option<String>,
}
