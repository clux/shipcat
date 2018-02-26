use std::path::Path;

use super::traits::Verify;
use super::Result;

/// Dependency of a service
///
/// We inject `{NAME}_ENDPOINT_API=kubeurl_to_service/api/{api}` as environment vars.
/// API contracts are used for testing as part of kube lifecycle hooks
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Dependency {
    /// Name of service relied upon (used to goto dependent manifest)
    pub name: String,
    /// API version relied upon (v1 default)
    pub api: Option<String>,
    /// Contract name for dependency
    pub contract: Option<String>,
    /// Protocol
    #[serde(default = "dependency_protocol_default")]
    pub protocol: String,
    /// Intent behind dependency - for manifest level descriptiveness
    pub intent: Option<String>,
}
fn dependency_protocol_default() -> String { "http".into() }


impl Verify for Dependency {
    fn verify(&self) -> Result<()> {
        if self.name == "core-ruby" || self.name == "php-backend-monolith" {
            debug!("Depending on legacy {} monolith", self.name);
            return Ok(())
        }
        // 5.a) self.name must exist in services/
        let dpth = Path::new(".").join("services").join(self.name.clone());
        if !dpth.is_dir() {
            bail!("Service {} does not exist in services/", self.name);
        }
        // 5.b) self.api must parse as an integer
        assert!(self.api.is_some(), "api version set by implicits");
        if let Some(ref apiv) = self.api {
            let vstr = apiv.chars().skip_while(|ch| *ch == 'v').collect::<String>();
            let ver : usize = vstr.parse()?;
            trace!("Parsed api version of dependency {} as {}", self.name.clone(), ver);
        }
        if self.protocol != "http" && self.protocol != "grpc" {
            bail!("Illegal dependency protocol {}", self.protocol)
        }
        Ok(())
    }
}
