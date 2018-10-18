use structs::resources::Resources;
use structs::traits::Verify;
use super::EnvVars;
use super::{Result, Config};

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct CronJob {
    /// The cron job name
    pub name: String,
    /// Schedule in Cron syntax
    pub schedule: String,
    /// Actual command to run as a sequence of arguments
    pub command: Vec<String>,

    /// CronJobs can use overridden images
    ///
    /// If they do, they must also specify the version of the images]
    pub image: Option<String>,

    /// Version to use for overridden images
    ///
    /// Only allowed if image is set.
    pub version: Option<String>,

    /// Resource limits and requests
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<Resources<String>>,

    /// Image to use for cron job
    /// Extra environment variables
    #[serde(default, skip_serializing_if = "EnvVars::is_empty")]
    pub extraEnv: EnvVars,
}


impl Verify for CronJob {
    fn verify(&self, conf: &Config) -> Result<()> {
        self.extraEnv.verify(conf)?;
        // TODO: name verify
        if let Some(ref r) = &self.resources {
            r.verify(conf)?;
        }

        Ok(())
    }
}
