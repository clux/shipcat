use super::{Resources};
use super::env::EnvVars;
use super::{Result};
use crate::deserializers::RelaxedString;

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub struct Sidecar {
  pub name: String,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub version: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub resources: Option<Resources<RelaxedString>>,

  #[serde(default)]
  pub env: EnvVars,
}

impl Sidecar {
    pub fn verify(&self) -> Result<()> {
      self.env.verify()?;
      Ok(())
    }
}
