use super::Container;

/// Restart policy
///
/// Used to decide if a job should be restarted when it fails or not.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RestartPolicy {
    Never,
    OnFailure,
}

impl Default for RestartPolicy {
    fn default() -> Self { RestartPolicy::Never }
}

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "filesystem", serde(deny_unknown_fields))]
pub struct JobVolumeClaim {
    /// The cron job name
    pub size: String,
    pub mountPath: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Job {
    /// Common properties for all types of container
    #[serde(flatten)]
    pub container: Container,

    /// Optional timeout, in seconds.
    /// u32 is enough; it'd fit a timeout 136 years in the future
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,

    /// Option to re-run the job on failure or not
    ///
    /// This defaults to none, but can be specified if wanted
    #[serde(default)]
    pub restartPolicy: RestartPolicy,

    pub volumeClaim: Option<JobVolumeClaim>,
}
