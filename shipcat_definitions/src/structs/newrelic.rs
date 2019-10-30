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
///       params:
///         threshold: "0.5"
///         priority: critical
///   incidentPreference: PER_POLICY
///   slack: C12ABYZ78
/// ```
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Newrelic {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub alerts: BTreeMap<String, NewrelicAlert>,
    pub incident_preference: NewrelicIncidentPreference,
    pub slack: SlackChannel,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewrelicAlert {
    pub name: String,
    pub template: String,
    pub params: BTreeMap<String, String>,
}

/// NewRelic AlertPolicy attribute that we configure once per Application (service@region) monitored
///
/// Details available at [this link](https://docs.newrelic.com/docs/alerts/new-relic-alerts/configuring-alert-policies/specify-when-new-relic-creates-incidents#preference-options)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NewrelicIncidentPreference {
    /// Only one incident will be open at a time for the entire policy. This is the default.
    ///
    ///  * Fewest number of alert notifications
    ///  * Requires immediate action and closing the incidents to be effective
    PerPolicy,
    /// One incident will be open at a time for each condition in your policy.
    ///
    ///  * More alert notifications
    ///  * Useful for policies containing conditions that focus on entities that
    ///    perform the same job; for example, hosts that all serve the same application(s)
    PerConditionAndTarget,
    /// An incident will be created for every violation in your policy.
    ///
    ///  * Most alert notifications
    ///  * Useful if you need to be notified of every violation or if you have an
    ///    external system where you want to send alert notifications
    PerCondition,
}

impl Default for NewrelicIncidentPreference {
    fn default() -> Self {
        NewrelicIncidentPreference::PerPolicy
    }
}
