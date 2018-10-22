use regex::Regex;

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

    /// Extra environment variables
    ///
    /// If this is used, rather than `env`, then all main environment variables
    /// are inherited (merged) in the chart.
    #[serde(default, skip_serializing_if = "EnvVars::is_empty")]
    pub extraEnv: EnvVars,

    /// Environment variables
    ///
    /// If this is used, rather than `extraEnv`, then the job ONLY gets these
    /// environment variables in the chart.
    #[serde(default, skip_serializing_if = "EnvVars::is_empty")]
    pub env: EnvVars,
}


impl Verify for CronJob {
    fn verify(&self, conf: &Config) -> Result<()> {
        let re = Regex::new(r"^[0-9a-z\-]{1,50}$").unwrap();
        if !re.is_match(&self.name) {
            bail!("Please use a short, lower case cron job names with dashes");
        }
        // TODO: version verify
        self.extraEnv.verify(conf)?;
        self.env.verify(conf)?;

        if let Some(ref r) = &self.resources {
            r.verify(conf)?;
        }
        if self.image.is_some() && self.version.is_none() {
            bail!("Cannot specify image without specifying version in CronJob")
        }
        if self.version.is_some() && self.image.is_none() {
            bail!("Cannot specify the version without specifying an image in CronJob")
        }
        if !self.env.is_empty() && !self.extraEnv.is_empty() {
            bail!("Cannot specify more than one of env and extraEnv in CronJob")
        }

        Ok(())
    }
}
