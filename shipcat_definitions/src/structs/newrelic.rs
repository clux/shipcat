use std::collections::BTreeMap;

use super::metadata::SlackChannel;

/// Monitoring section covering NewRelic configuration
///
/// ```yaml
/// newrelic:
///   alerts:
///     alert_name_foo:
///       name: alert_name_foo:
///       template: appdex
///       slack: C12ABYZ78
///       params:
///         threshold: "0.5"
///         priority: critical
/// ```
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Newrelic {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub alerts: BTreeMap<String, NewrelicAlert>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewrelicAlert {
    pub name: String,
    pub template: String,
    pub slack: SlackChannel,
    pub params: BTreeMap<String, String>,
}
