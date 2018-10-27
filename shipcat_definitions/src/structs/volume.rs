use super::traits::Verify;
use super::{Result, Config};
use std::collections::BTreeMap;

// These structs contain a straight translation of kubernetes volumes
// TODO: cross reference better with
// https://kubernetes.io/docs/concepts/storage/volumes/

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
pub struct ProjectedVolumeSecretSource {
    pub secret: ProjectedVolumeSecretSourceDetail,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ProjectedVolumeSecret {
    pub sources: Vec<ProjectedVolumeSecretSource>,
    // pub default_mode: u32,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct DownwardApiWrapper {
    pub items: Vec<DownwardApiItem>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DownwardApiItem {
    /// Kube path to string
    pub path: String,
    /// Specific kube paths to values
    pub resourceFieldRef: DownWardApiResource,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DownWardApiResource {
    /// Name of container TODO: default to service name
    pub containerName: String,
    /// Raw accesssor, e.g. limits.cpu, status.podIP, etc TODO: validate
    pub resource: String,
    /// Format resource is returned in (defaults to 1 if missing), can set to 1m
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub divisor: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Volume {
    pub name: String,
    /// A projection combines multiple volume items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projected: Option<ProjectedVolumeSecret>,
    /// The secret is fetched from kube secrets and mounted as a volume
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<VolumeSecretDetail>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emptyDir: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub persistentVolumeClaim: BTreeMap<String, String>,
    /// Items from the Downward API
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downwardAPI: Option<DownwardApiWrapper>,
}

impl Verify for Volume {
    fn verify(&self, _: &Config) -> Result<()> {
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
