#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct CronJob {
    /// The cron job name
    pub name: String,
    /// Schedule in Cron syntax
    pub schedule: String,
    /// Actual command to run as a sequence of arguments
    pub command: Vec<String>,
}
