use regex::Regex;

use shipcat_definitions::Result;
use shipcat_definitions::structs::port::{Port, PortProtocol};

use crate::util::{Build};

#[derive(Deserialize, Clone, Default)]
pub struct PortName(String);

impl Build<String, ()> for PortName {
    fn build(self, _: &()) -> Result<String> {
        let Self(name) = self;
        // https://github.com/kubernetes/community/blob/master/contributors/design-proposals/architecture/identifiers.md#definitions
        // https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.14/#containerport-v1-core
        let re = Regex::new(r"^[a-z0-9-]{1,15}$").unwrap();
        if !re.is_match(&name) {
            bail!("Port names must be 1-15 lowercase alphanumeric characters or hyphens");
        }

        Ok(name)
    }
}

#[derive(Deserialize, Clone, Default)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct PortSource {
    /// Name of the port
    pub name: PortName,
    /// Port to open
    pub port: u32,
    /// Port to expose on K8s service
    pub service_port: Option<u32>,
    /// Port protocol
    pub protocol: Option<PortProtocol>,
}

impl Build<Port, ()> for PortSource {
    fn build(self, _: &()) -> Result<Port> {
        Ok(Port {
            name: self.name.build(&())?,
            port: self.port,
            service_port: self.service_port.unwrap_or(self.port),
            protocol: self.protocol.unwrap_or_default(),
        })
    }
}
