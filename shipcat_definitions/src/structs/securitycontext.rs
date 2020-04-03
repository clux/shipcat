/// Security context for ownership of volumes
///
/// Verbatim from [kubernetes SecurityContext](https://kubernetes.io/docs/tasks/configure-pod-container/security-context/#configure-volume-permission-and-ownership-change-policy-for-pods)
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct SecurityContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runAsUser: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runAsGroup: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fsGroup: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fsGroupChangePolicy: Option<String>,
}
