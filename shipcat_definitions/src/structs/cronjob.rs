use super::Container;
use super::job::JobVolumeClaim;

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
}
