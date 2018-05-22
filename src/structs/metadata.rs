use super::traits::Verify;
use super::{Config, Result};

/// Contact data
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Contact {
    /// Free text name
    pub name: String,
    /// Slack handle
    pub slack: String,
}

/// Metadata for a service
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    /// Git repository
    pub repo: String,
    /// Owning team
    pub team: String,
    /// Contact person
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contacts: Vec<Contact>,
    /// Support channels
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub support: Vec<String>,
    /// Canoncal documentation link
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
    // TODO: generate swagger docs url from region and service name
}

impl Verify for Metadata {
    fn verify(&self, conf: &Config) -> Result<()> {
        let teams = conf.teams.clone().into_iter().map(|t| t.name).collect::<Vec<_>>();
        if !teams.contains(&self.team) {
            bail!("Illegal team name {} not found in the config", self.team);
        }
        for cc in &self.contacts {
            if cc.name.is_empty() {
                bail!("Contact name cannot be empty")
            }
            if !cc.slack.starts_with("@") {
                bail!("Contact slack handle needs to start with the slack guid '@U...' - got {}", cc.slack)
            }
            if cc.slack.contains("|") {
                bail!("Contact slack user id invalid - got {}", cc.slack)
            }
        }

        Ok(())
    }
}
