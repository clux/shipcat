use super::{Resources, Probe, Port, EnvVars};
use super::autoscaling::AutoScaling;

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
    pub resources: Resources<String>,
    /// Replication limits
    pub replicaCount: u32,

    /// Autoscaling parameters
    ///
    /// Overrides the replicaCount for this worker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoScaling: Option<AutoScaling>,

    /// Extra environment variables
    #[serde(default, skip_serializing_if = "EnvVars::is_empty")]
    pub extraEnv: EnvVars,

    /// Http Port to expose
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub httpPort: Option<u32>,
    /// Ports to open
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<Port>,
    /// Optional readiness probe
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readinessProbe: Option<Probe>,
    /// Optional liveness probe
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub livenessProbe: Option<Probe>,
}

impl Verify for Worker {
    fn verify(&self, conf: &Config) -> Result<()> {
        self.extraEnv.verify(conf)?;
        for p in &self.ports {
            p.verify(&conf)?;
        }
        self.resources.verify(conf)?;

        // maybe the http ports shouldn't overlap? might not matter.
        Ok(())
    }
}
