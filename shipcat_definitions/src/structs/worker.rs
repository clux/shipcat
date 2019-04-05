use super::{Resources, Probe, Port, EnvVars};
use super::autoscaling::AutoScaling;
use crate::deserializers::{RelaxedString};

use super::Result;


/// Worker for a service
///
/// Essentially a side-car like object that can scale resources separately to the main pods.
/// Useful for services that have one single side service that polls or does some work.
#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(filesystem, serde(deny_unknown_fields))]
pub struct Worker {
    /// Name of the worker
    pub name: String,

    /// Image command override
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,

    /// Resource limits and requests
    pub resources: Resources<RelaxedString>,
    /// Replication limits
    pub replicaCount: u32,

    /// Autoscaling parameters
    ///
    /// Overrides the replicaCount for this worker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoScaling: Option<AutoScaling>,

    /// Environment variables for the workers
    ///
    /// These may be specified in addition to the main deployment `env` vars
    /// or as fresh variables, depending on `preserveEnv`.
    #[serde(default)]
    pub env: EnvVars,

    /// Add environment variables from parent deployment into this worker
    ///
    /// This is off by default, which means you specify all the environment variables
    /// you need for this worker in the corresponding `worker.env`.
    #[serde(default)]
    pub preserveEnv: bool,

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

impl Worker {
    pub fn verify(&self) -> Result<()> {
        self.env.verify()?;
        if let Some(hpa) = &self.autoScaling {
            hpa.verify()?;
        }
        for p in &self.ports {
            p.verify()?;
        }
        self.resources.verify()?;
        if let Some(rp) = &self.readinessProbe {
            rp.verify()?;
        }
        if let Some(lp) = &self.livenessProbe {
            lp.verify()?;
        }

        // maybe the http ports shouldn't overlap? might not matter.
        Ok(())
    }
}
