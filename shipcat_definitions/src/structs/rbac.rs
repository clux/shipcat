use super::Result;

/// RBAC (Role-Based Access Control) PolicyRule
///
/// Designed for services which requires escalated privileges
/// Used to generate roles and role bindings in kubernetes
///
/// This is a port of [k8s PolicyRule](https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.15/#policyrule-v1beta1-rbac-authorization-k8s-io)
/// We skip `nonResourceURLs` since it is only relevant for ClusterRoles
/// We also disallow empty resources to shoehorn in "all" access.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "filesystem", serde(deny_unknown_fields))]
pub struct Rbac {
    /// API groups containing resources
    pub apiGroups: Vec<String>,
    /// Resources on which to apply verbs / actions
    pub resources: Vec<String>,
    /// Optional white list of names that the rule applies to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resourceNames: Vec<String>,
    /// Actions to be allowed
    pub verbs: Vec<String>,
}

impl Rbac {
    pub fn verify(&self) -> Result<()> {
        if self.apiGroups.is_empty() {
            bail!("RBAC needs to have at least one item in apiGroups");
        }
        if self.resources.is_empty() {
            bail!("RBAC needs to have at least one item in resources");
        }
        if self.verbs.is_empty() {
            bail!("RBAC needs to have at least one item in verbs");
        }

        Ok(())
    }
}
