use super::{Resources};
use super::env::EnvVars;
use super::{Result};

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub struct Sidecar {
  pub name: String,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub version: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub resources: Option<Resources<String>>,

  #[serde(default, skip_serializing_if = "EnvVars::is_empty")]
  pub env: EnvVars,
}

impl Sidecar {
    pub fn verify(&self) -> Result<()> {
      self.env.verify()?;
      Ok(())
    }
}
