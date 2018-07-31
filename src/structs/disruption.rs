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

/// PodDisruptionBudget representation
///
/// Users need to set exactly one of these to pass validation.
/// The values are "how many replicas" when integer values are used,
/// and "what percentage of total replicas" when a % is added to the string.
#[derive(Serialize, Deserialize, Clone)]
pub struct DisruptionBudget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minAvailable: Option<AvailabilityPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maxUnavailable: Option<AvailabilityPolicy>,
}

impl DisruptionBudget {
     pub fn verify(&self, replicas: u32) -> Result<()> {
        if self.minAvailable.is_none() && self.maxUnavailable.is_none() {
            bail!("Need to set one of minAvailable or maxUnavailable in disruptionBudget");
        }
        if self.minAvailable.is_some() && self.maxUnavailable.is_some() {
            bail!("Cannot set both minAvailable and maxUnavailable in disruptionBudget");
        }
        if let Some(ref ma) = &self.minAvailable {
            ma.verify("minAvailable", replicas)?;
        }
        if let Some(ref mu) = &self.maxUnavailable {
            mu.verify("maxUnavailable", replicas)?;
        }
        Ok(())
     }
}
