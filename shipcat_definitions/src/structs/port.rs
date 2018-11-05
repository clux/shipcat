use super::Result;

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
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Port {
    /// Name of the port
    pub name: String,
    /// Port to open
    pub port: u32,
    /// Port protocol
    #[serde(default)]
    pub protocol: PortProtocol,
}

impl Port {
    pub fn verify(&self) -> Result<()> {
        assert_ne!(self.port, 80, "Port should not be 80");
        Ok(())
    }
}
