use std::path::Path;

use super::traits::Verify;
use super::{Config, Result};


/// Supported dependency protocols
///
/// Forces lowercase values of this enum to be used
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DependencyProtocol {
    /// HTTP REST dependency
    Http,
    /// GRPC dependency
    Grpc,
    /// Kafka communication based dependency
    Kafka,
    /// RabbitMQ style dependency
    Amqp,
    /// Amazon SQS style dependency
    Sqs,
}
impl Default for DependencyProtocol {
    fn default() -> DependencyProtocol {
        DependencyProtocol::Http
    }
}

/// Dependency of a service
///
/// We inject `{NAME}_ENDPOINT_API=kubeurl_to_service/api/{api}` as environment vars.
/// API contracts are used for testing as part of kube lifecycle hooks
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    /// Name of service relied upon (used to goto dependent manifest)
    pub name: String,
    /// API version relied upon (v1 default)
    pub api: Option<String>,
    /// Contract name for dependency
    pub contract: Option<String>,
    /// Protocol/message passing service used to depend on a service
    #[serde(default)]
    pub protocol: DependencyProtocol,
    /// Intent behind dependency - for manifest level descriptiveness
    pub intent: Option<String>,
}

impl Verify for Dependency {
    fn verify(&self, _: &Config) -> Result<()> {
        // self.name must exist in services/
        let dpth = Path::new(".").join("services").join(self.name.clone());
        if !dpth.is_dir() {
            bail!("Service {} does not exist in services/", self.name);
        }
        // self.api must parse as an integer
        assert!(self.api.is_some(), "api version set by implicits");
        if let Some(ref apiv) = self.api {
            let vstr = apiv.chars().skip_while(|ch| *ch == 'v').collect::<String>();
            let ver : usize = vstr.parse()?;
            trace!("Parsed api version of dependency {} as {}", self.name.clone(), ver);
        }
        Ok(())
    }
}
