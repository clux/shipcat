/// Dependency of a service
///
/// We inject `{NAME}_ENDPOINT_API=kubeurl_to_service/api/{api}` as environment vars.
/// API contracts are used for testing as part of kube lifecycle hooks
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Dependency {
    /// Name of service relied upon (used to goto dependent manifest)
    pub name: String,
    /// API version relied upon (v1 default)
    pub api: Option<String>,
    /// Contract name for dependency
    pub contract: Option<String>,
    /// Protocol
    #[serde(default = "dependency_protocol_default")]
    pub protocol: String,
    /// Intent behind dependency - for manifest level descriptiveness
    pub intent: Option<String>,
}
fn dependency_protocol_default() -> String { "http".into() }
