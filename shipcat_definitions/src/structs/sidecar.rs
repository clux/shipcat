use super::{Resources};

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub struct Sidecar {
  pub name: String,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub version: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub resources: Option<Resources<String>>,
}
