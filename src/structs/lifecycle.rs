use structs::traits::Verify;
use super::{Result, Config};

/// A straight port of Kubernetes Container Lifecycle Events
///
/// From https://kubernetes.io/docs/tasks/configure-pod-container/attach-handler-lifecycle-event/
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct LifeCycle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postStart: Option<LifeCycleHandler>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preStop: Option<LifeCycleHandler>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct LifeCycleHandler {
   pub  exec: ExecHandler,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ExecHandler {
    command: Vec<String>,
}

impl Verify for LifeCycle {
    fn verify(&self, conf: &Config) -> Result<()> {
        if self.postStart.is_none() && self.preStop.is_none() {
            bail!("Need to set one of postStart or preStop in lifecycle");
        }
        if self.postStart.is_some() && self.preStop.is_some() {
            bail!("Cannot set both postStart and preStop in lifecycle");
        }
        if let Some(ref start) = self.postStart {
            start.verify(conf)?;
        }
        if let Some(ref stop) = self.preStop {
            stop.verify(conf)?;
        }
        Ok(())
    }
}

impl Verify for LifeCycleHandler {
    fn verify(&self, _conf: &Config) -> Result<()> {
        if self.exec.command.is_empty() {
            bail!("Cannot have empty lifecycle exec commands");
        }
        Ok(())
    }
}
