use super::Result;

/// RBAC (Role-Based Access Control)
///
/// Designed for services which requires escalated privileges
/// Used to generate roles and role bindings in kubernetes
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Rbac {
    /// API groups containing resources (defined below)
    pub apiGroups: Vec<AllowedApiGroups>,
    /// Resources on which to apply verbs / actions
    pub resources: Vec<AllowedResources>,
    /// Actions to be allowed
    pub verbs: Vec<AllowedVerbs>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum AllowedApiGroups {
    #[serde(rename = "")]
    Empty,
    Extensions,
    Batch,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum AllowedResources {
    Deployments,
    ReplicaSets,
    Jobs,
    CronJobs,
    Pods,
    #[serde(rename = "pods/log")]
    PodsSlashLog,
    ConfigMaps,
    Namespaces,
    HorizontalPodAutoscaler,
    Events,
    Nodes,
    RoleBindings,
    Roles,
    Secrets,
    ServiceAccounts,
    Services,
    ShipcatManifest,
    ShipcatConfig,
}

/// We don't allow eg Delete or other operations for security reasons (least privilege).
/// More operations can be added if required but due diligence would be sane.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum AllowedVerbs {
    List,
    Get,
    Watch,
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
