use std::collections::BTreeMap;
use super::{Resources, Probe};

use super::traits::Verify;
use super::{Config, Result};


/// Worker for a service
///
/// Essentially a side-car like object that can scale resources separately to the main pods.
/// Useful for services that have one single side service that polls or does some work.
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Worker {
    /// Name of the worker
    pub name: String,

    /// Image command override
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,

    /// Resource limits and requests
    pub resources: Resources,
    /// Replication limits
    pub replicaCount: u32,

    /// Extra environment variables
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extraEnv: BTreeMap<String, String>,

    /// Http Port to expose
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub httpPort: Option<u32>,
    /// Optional readiness probe
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readinessProbe: Option<Probe>,
    /// Optional liveness probe
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub livenessProbe: Option<Probe>,
}

impl Verify for Worker {
    fn verify(&self, _: &Config) -> Result<()> {
        for (k, v) in &self.extraEnv {
            if v == "IN_VAULT" {
                bail!("Secret evars must go in the root service (included in workers)");
            }
            if k != &k.to_uppercase()  {
                bail!("Env vars need to be uppercase, found: {}", k);
            }
        }
        // maybe the http ports shouldn't overlap? might not matter.
        Ok(())
    }
}
