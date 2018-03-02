use serde_yaml::Sequence;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct CronJob {
    /// The cron job name
    pub name: String,
    /// Schedule in Cron syntax
    pub schedule: String,
    /// Actual command to run as a sequence of arguments
    pub command: Sequence,
}
