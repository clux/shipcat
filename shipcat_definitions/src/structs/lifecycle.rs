use super::Result;

/// A straight port of Kubernetes Container Lifecycle Events
///
/// From https://kubernetes.io/docs/tasks/configure-pod-container/attach-handler-lifecycle-event/
#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(filesystem, serde(deny_unknown_fields))]
pub struct LifeCycle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postStart: Option<LifeCycleHandler>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preStop: Option<LifeCycleHandler>,
}

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(filesystem, serde(deny_unknown_fields))]
pub struct LifeCycleHandler {
   pub exec: ExecAction,
}

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(filesystem, serde(deny_unknown_fields))]
pub struct ExecAction {
    command: Vec<String>,
}

// TODO: support HttpGetAction + TcpSocketAction

impl LifeCycle {
    pub fn verify(&self) -> Result<()> {
        if self.postStart.is_none() && self.preStop.is_none() {
            bail!("Need to set one of postStart or preStop in lifecycle");
        }
        if self.postStart.is_some() && self.preStop.is_some() {
            bail!("Cannot set both postStart and preStop in lifecycle");
        }
        if let Some(ref start) = self.postStart {
            start.verify()?;
        }
        if let Some(ref stop) = self.preStop {
            stop.verify()?;
        }
        Ok(())
    }
}

impl LifeCycleHandler {
    pub fn verify(&self) -> Result<()> {
        if self.exec.command.is_empty() {
            bail!("Cannot have empty lifecycle exec commands");
        }
        Ok(())
    }
}
