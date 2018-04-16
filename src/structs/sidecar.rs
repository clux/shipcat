use super::{Resources};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub struct Sidecar {
  pub name: SidecarName,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub resources: Option<Resources>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SidecarName {
  /// Redis sidecar
  Redis,
}
