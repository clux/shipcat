use super::{EnvVars, Port, Probe, ResourceRequirements, VolumeMount};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Container {
    /// Name of container
    pub name: String,

    /// Docker image name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Docker image tag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Resource Requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceRequirements<String>>,

    /// Command override
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,
    /// Environment variables
    pub env: EnvVars,

    /// Readiness probe
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readiness_probe: Option<Probe>,
    /// Liveness probe
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub liveness_probe: Option<Probe>,

    /// Ports to open
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<Port>,

    /// Volume mounts
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volume_mounts: Vec<VolumeMount>,
}
