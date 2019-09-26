#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PortProtocol {
    Tcp,
    Udp,
    Sctp,
}

impl Default for PortProtocol {
    fn default() -> Self { PortProtocol::Tcp }
}

/// Port to open on a container
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Port {
    /// Name of the port
    pub name: String,
    /// Port to open
    pub port: u32,
    /// Port to expose on K8s service
    pub service_port: u32,
    /// Port protocol
    #[serde(default)]
    pub protocol: PortProtocol,
}
