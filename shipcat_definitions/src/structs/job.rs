use regex::Regex;

use semver::Version;

use super::EnvVars;
use super::Result;

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
#[cfg_attr(feature = "filesystem", serde(deny_unknown_fields))]
pub struct Job {
    /// The job name
    pub name: String,
    /// Actual command to run as a sequence of arguments
    pub command: Vec<String>,

    /// Jobs can use overridden images
    ///
    /// If they do, they must also specify the version of the image
    pub image: Option<String>,

    /// Version to use for overridden images
    ///
    /// Only allowed if image is set.
    pub version: Option<String>,

    /// Volume claim for this job if it needs local scratch space
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volumeClaim: Option<JobVolumeClaim>,

    /// Optional timeout, in seconds.
    /// u32 is enough; it'd fit a timeout 136 years in the future
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,

    /// Environment variables for the jobs
    ///
    /// These may be specified in addition to the main deployment `env` vars
    /// or as fresh variables, depending on `preserveEnv`.
    #[serde(default)]
    pub env: EnvVars,

    /// Add environment variables from parent deployment into this job
    ///
    /// This is off by default, which means you specify all the environment variables
    /// you need for this job in the corresponding `job.env`.
    #[serde(default)]
    pub preserveEnv: bool,

    /// Option to re-run the job on failure or not
    ///
    /// This defaults to none, but can be specified if wanted
    #[serde(default)]
    pub restartPolicy: RestartPolicy
}


impl Job {
    pub fn verify(&self) -> Result<()> {
        let re = Regex::new(r"^[0-9a-z\-]{1,50}$").unwrap();
        if !re.is_match(&self.name) {
            bail!("Please use a short, lower case job names with dashes");
        }
        if let Some(v) = &self.version {
            if Version::parse(&v).is_err() {
                bail!("Please use a valid semver tag for the image version");
            }
        }

        self.env.verify()?;

        if self.image.is_some() && self.version.is_none() {
            bail!("Cannot specify image without specifying version in Job")
        }
        if self.version.is_some() && self.image.is_none() {
            bail!("Cannot specify the version without specifying an image in Job")
        }

        Ok(())
    }
}
