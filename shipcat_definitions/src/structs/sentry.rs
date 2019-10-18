use super::metadata::SlackChannel;

/// Monitoring section covering Sentry configurations
///
/// ```yaml
/// sentry:
///   slack: C12ABYZ78
///   silent: true
///   dsnEnvName: SENTRY_DSN_CUSTOM
/// ```
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sentry {
    pub slack: SlackChannel,
    pub silent: bool,
    pub dsn_env_name: String,
}
