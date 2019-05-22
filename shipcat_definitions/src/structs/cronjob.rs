use super::Container;
use super::job::JobVolumeClaim;
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct CronJob {
    /// Common properties for all types of container
    #[serde(flatten)]
    pub container: Container,

    /// Schedule in Cron syntax
    pub schedule: String,

    /// Volume claim for this job if it needs local scratch space
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volumeClaim: Option<JobVolumeClaim>,

    /// Optional timeout, in seconds.
    /// u32 is enough; it'd fit a timeout 136 years in the future
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,

    /// Metadata Annotations for pod spec templates in cron jobs
    ///
    /// https://kubernetes.io/docs/concepts/overview/working-with-objects/annotations/
    ///
    /// ```yaml
    /// podAnnotations:
    ///   iam.amazonaws.com/role: role-arn
    /// ```
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub podAnnotations: BTreeMap<String, String>,
}
