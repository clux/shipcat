use super::traits::Verify;
use super::{Config, Result};

/// Metadata for a service
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Metadata {
    /// Git repository
    pub repo: String,
    /// Owning team
    pub team: String,
    /// Contact person
    pub contacts: Vec<String>,
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
            let split: Vec<&str> = cc.split('|').collect();
            if split.len() != 2 {
                bail!("Contact needs to be of the form @U82SKDQD9|clux - got {}", cc);
            }
            if !split[0].starts_with("@U") {
                bail!("Contact need to start with the slack guid '@U...'")
            }
        }

        Ok(())
    }
}
