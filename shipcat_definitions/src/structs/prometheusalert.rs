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
    pub name: String,

    /// Summary of the alert.
    ///
    /// A one-line summary of the problem this alert captures.
    pub summary: String,

    /// Description of the alert.
    ///
    /// A more verbose description of the problem should go here, together with any suggested actions
    /// or links to further resources useful to an on-call engineer responding to this issue.
    pub description: String,

    /// PromQL expression defining this alert.
    ///
    /// Whenever a new timeseries is returned by this expression, a new alert enters pending state.
    pub expr: String,

    /// Minimum duration of a problem before an alert fires.
    ///
    /// This is the minimum duration a pending alert must remain active for in order to actually fire.
    /// Examples: '15m', '1h'.
    pub min_duration: String,

    /// Severity of the alert.
    ///
    /// Corresponds to how urgently it should be actioned if it were in production.
    pub severity: PrometheusAlertSeverity,
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
    pub fn verify(&self, svc: &str) -> Result<()> {
        if !is_pascal_case(&self.name) {
            bail!("Prometheus alert for {} needs a non-empty PascalCaseName", svc);
        }
        if self.summary.is_empty() {
            bail!(
                "Prometheus alert for {} needs a summary of the problem it identifies",
                svc
            );
        }
        if self.description.is_empty() {
            bail!(
                "Prometheus alert for {} needs a description of the problem it identifies",
                svc
            );
        }
        if !Regex::new(r"^\d+[mh]$").unwrap().is_match(&self.min_duration) {
            bail!("Prometheus alert has invalid min_duration value (needs to be like '15m' or '1h')");
        }
        // PromQL expression sanity (NB: syntax only, operator verifies properly)
        if let Err(e) = prometheus_parser::parse_expr(&self.expr) {
            bail!("Prometheus alert expression for {} invalid: {:?}", svc, e);
        }

        Ok(())
    }
}
