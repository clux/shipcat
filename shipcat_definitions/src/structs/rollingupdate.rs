use super::{Result};

// Untagged enum to get around the weird validation
#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AvailabilityPolicy {
    Percentage(String),
    Unsigned(u32),
}
// Kube has a weird hybrid type for this intstr.IntOrString: IntVal | StrVal
// if it's a string, then '[0-9]+%!' has to parse
impl AvailabilityPolicy {
    fn verify(&self, name: &str, maxNumber: u32) -> Result<()> {
        match self {
            AvailabilityPolicy::Unsigned(ref n) => {
                if n > &maxNumber {
                    bail!("Cannot have {} set higher than replicaCount {}", name, maxNumber);
                }
            },
            AvailabilityPolicy::Percentage(s) => {
                if !s.ends_with('%') {
                    bail!("{} must end with a '%' sign", name);
                }
                let digits = s.chars().take_while(|ch| *ch != '%').collect::<String>();
                let res : u32 = digits.parse()?;
                if res > 100 {
                    bail!("Percentage value for {} cannot exceed 100", name);
                }
            }
        }
        Ok(())
    }
}

/// Configuration parameters for Deployment.spec.strategy.rollingUpdate
#[derive(Serialize, Deserialize, Clone)]
pub struct RollingUpdate {
    /// How many replicas or percentage of replicas that can be down during rolling-update
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maxUnavailable: Option<AvailabilityPolicy>,
    /// Maximum number of pods that can be created over replicaCount
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maxSurge: Option<AvailabilityPolicy>,
}

/// Implement Default that matches kubernetes
///
/// Both values defined in kube docs for deployment under .spec.strategy
/// https://kubernetes.io/docs/concepts/workloads/controllers/deployment/#writing-a-deployment-spec
impl Default for RollingUpdate {
    fn default() -> Self {
        RollingUpdate {
            maxUnavailable: Some(AvailabilityPolicy::Percentage(25.to_string())),
            maxSurge: Some(AvailabilityPolicy::Percentage(25.to_string())),
        }
    }
}


impl RollingUpdate {
     pub fn verify(&self, replicas: u32) -> Result<()> {
        if self.maxUnavailable.is_none() && self.maxSurge.is_none() {
            bail!("Need to set one of maxUnavailable or maxSurge in rollingUpdate");
        }
        if let Some(ref ma) = &self.maxUnavailable {
            ma.verify("maxUnavailable", replicas)?;
        }
        if let Some(ref mu) = &self.maxSurge {
            mu.verify("maxSurge", replicas)?;
        }
        Ok(())
     }
}


impl RollingUpdate {
    /// Estimate how many cycles is needed to roll out a new version
    ///
    /// This is a bit arcane extrapolates from [rolling update documentation](https://kubernetes.io/docs/concepts/workloads/controllers/deployment/#max-unavailable)
    /// It needs to keep into account both values.
    pub fn rollout_iterations(&self, replicas: u32) -> u32 {
        if let Some(surge) = self.maxSurge {
            // surge is max number/percentage
            match surge {
                AvailabilityPolicy::Percentage(percstr) => {
                    let digits = percstr.chars().take_while(|ch| *ch != '%').collect::<String>();
                    let surgeperc : u32 = digits.parse().unwrap(); // safe due to verify ^
                    ((replicas as f64 * surgeperc as f64)/ 100.0).ceil() as u32
                },
                AvailabilityPolicy::Unsigned(u) => {
                    (replicas as f64 / u as f64).ceil() as u32
                }
            }
        } else {
            RollingUpdate::rollout_iterations_default(replicas)
        }

    }
    pub fn rollout_iterations_default(replicas: u32) -> u32 {
        // default surge percentage is 25
        ((replicas as f64 * 25 as f64)/ 100.0).ceil() as u32
    }
}


#[cfg(test)]
mod tests {
    use super::RollingUpdate;

    #[test]
    fn rollout_iteration_check() {
        let ru = RollingUpdate::default();
        let iters = ru.rollout_iterations(8);
        assert_eq!()

        assert!(res.is_ok());
        let ru = res.unwrap();
        assert_eq!(ru, "prefix-0.1.2-suffix")
    }
}
