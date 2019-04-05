use regex::Regex;

use crate::structs::resources::Resources;
use crate::deserializers::RelaxedString;
use super::EnvVars;
use super::Result;

#[derive(Serialize, Deserialize, Clone, Default)]
#[cfg_attr(filesystem, serde(deny_unknown_fields))]
pub struct CronJobVolumeClaim {
    /// The cron job name
    pub size: String,
    pub mountPath: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[cfg_attr(filesystem, serde(deny_unknown_fields))]
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
    pub resources: Option<Resources<RelaxedString>>,

    /// Volume claim for this job if it needs local scratch space
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volumeClaim: Option<CronJobVolumeClaim>,

    /// Optional timeout, in seconds.
    /// u32 is enough; it'd fit a timeout 136 years in the future
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,

    /// Environment variables for the workers
    ///
    /// These may be specified in addition to the main deployment `env` vars
    /// or as fresh variables, depending on `preserveEnv`.
    #[serde(default)]
    pub env: EnvVars,

    /// Add environment variables from parent deployment into this worker
    ///
    /// This is off by default, which means you specify all the environment variables
    /// you need for this worker in the corresponding `worker.env`.
    #[serde(default)]
    pub preserveEnv: bool,
}


impl CronJob {
    pub fn verify(&self) -> Result<()> {
        let re = Regex::new(r"^[0-9a-z\-]{1,50}$").unwrap();
        if !re.is_match(&self.name) {
            bail!("Please use a short, lower case cron job names with dashes");
        }
        // TODO: version verify
        self.env.verify()?;

        if let Some(ref r) = &self.resources {
            r.verify()?;
        }
        if self.image.is_some() && self.version.is_none() {
            bail!("Cannot specify image without specifying version in CronJob")
        }
        if self.version.is_some() && self.image.is_none() {
            bail!("Cannot specify the version without specifying an image in CronJob")
        }

        Ok(())
    }
}
