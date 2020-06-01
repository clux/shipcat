use super::Result;
use inflector::cases::pascalcase::is_pascal_case;
use regex::Regex;

/// Data describing one Prometheus alert.
///
/// This roughly corresponds to a Rule object in the Prometheus Operator API spec:
/// https://github.com/coreos/prometheus-operator/blob/master/Documentation/api.md#rule
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrometheusAlert {
    /// Name of the alert
    ///
    /// Must be in PascalCase.
    name: String,

    /// Summary of the alert.
    ///
    /// A one-line summary of the problem this alert captures.
    summary: String,

    /// Description of the alert.
    ///
    /// A more verbose description of the problem should go here, together with any suggested actions
    /// or links to further resources useful to an on-call engineer responding to this issue.
    description: String,

    /// PromQL expression defining this alert.
    ///
    /// Whenever a new timeseries is returned by this expression, a new alert enters pending state.
    expr: String,

    /// Minimum duration of a problem before an alert fires.
    ///
    /// This is the minimum duration a pending alert must remain active for in order to actually fire.
    /// Examples: '15m', '1h'.
    min_duration: String,

    /// Severity of the alert.
    ///
    /// Corresponds to how urgently it should be actioned if it were in production.
    severity: PrometheusAlertSeverity,
}

/// Alert severity enumeration.
///
/// Represents the set of alert severities we allow in our Prometheus alerts.
#[serde(rename_all = "lowercase")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PrometheusAlertSeverity {
    /// Warning severity
    ///
    /// Low urgency, should not wake on-call engineers and should not escalate.
    Warning,

    /// Error severity
    ///
    /// High urgency, should wake up on-call engineers and also escalate if unacknowledged
    Error,
}

impl PrometheusAlert {
    pub fn verify(&self) -> Result<()> {
        if !is_pascal_case(&self.name) {
            bail!("Prometheus alert needs a non-empty PascalCaseName");
        }
        if self.summary.is_empty() {
            bail!("Prometheus alert needs a summary of the problem it identifies");
        }
        if self.description.is_empty() {
            bail!("Prometheus alert needs a description of the problem it identifies");
        }
        // Validation of the PromQL should be done by Prometheus Operator
        if self.expr.is_empty() {
            bail!("Prometheus alert needs an expr (PromQL) to evaluate");
        }
        if !Regex::new(r"^\d+[mh]$").unwrap().is_match(&self.min_duration) {
            bail!("Prometheus alert has invalid min_duration value (needs to be like '15m' or '1h')");
        }

        Ok(())
    }
}
