use super::traits::Verify;
use super::Result;
use std::collections::BTreeMap;

// These structs contain a straight translation of kubernetes volumes

#[derive(Serialize, Deserialize, Clone)]
pub struct VolumeSecretItem {
    #[serde(default = "volume_key")]
    pub key: String,
    pub path: String,
    #[serde(default = "volume_default_mode")]
    pub mode: u32,
}
fn volume_key() -> String { "value".into() }
fn volume_default_mode() -> u32 { 420 } // 0644

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct VolumeSecretDetail {
    pub secretName: String,
    pub items: Vec<VolumeSecretItem>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ProjectedVolumeSecretSourceDetail {
    pub name: String,
    pub items: Vec<VolumeSecretItem>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct VolumeSecret {
    pub secret: Option<VolumeSecretDetail>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ProjectedVolumeSecretSource {
    pub secret: Option<VolumeSecretSourceDetail>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ProjectedVolumeSecret {
    pub sources: Vec<ProjectedVolumeSecretSource>,
    // pub default_mode: u32,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Volume {
    pub name: String,
    /// A projection combines multiple volume items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projected: Option<ProjectedVolumeSecret>,
    /// The secret is fetched  from kube secrets and mounted as a volume
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<VolumeSecretDetail>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emptyDir: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub persistentVolumeClaim: BTreeMap<String, String>,
}

impl Verify for Volume {
    fn verify(&self) -> Result<()> {
        // TODO: verify stuff here
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct VolumeMount {
    pub name: String,
    pub mountPath: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subPath: Option<String>,
    #[serde(default = "volume_mount_read_only")]
    pub readOnly: bool,
}
fn volume_mount_read_only() -> bool { false }
