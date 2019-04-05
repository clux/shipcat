use std::ops::{Add, AddAssign, Mul};
use super::Result;
use crate::deserializers::{RelaxedString};

// Kubernetes resouce structs
//
// These are used in manifests where T is a String
// but is generic herein because we can have a fully parsed version
// where all values are parsed as normalised f64s.
// This allows extra computation, and certain versions will have some extra traits
// implemented to be a bit more useful, as well as some to convert between them.

/// Kubernetes resource requests
#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(filesystem, serde(deny_unknown_fields))]
pub struct ResourceRequest<T> {
    /// CPU request string
    pub cpu: T,
    /// Memory request string
    pub memory: T,
    // TODO: ephemeral-storage + extended-resources
}

/// Kubernetes resource limits
#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(filesystem, serde(deny_unknown_fields))]
pub struct ResourceLimit<T> {
    /// CPU limit string
    pub cpu: T,
    /// Memory limit string
    pub memory: T,
    // TODO: ephemeral-storage + extended-resources
}

/// Kubernetes resources
///
/// This can be inlined straight into a container spec at the moment
#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(filesystem, serde(deny_unknown_fields))]
pub struct Resources<T> {
    /// Resource requests for k8s
    pub requests: ResourceRequest<T>,
    /// Resource limits for k8s
    pub limits: ResourceLimit<T>,
}

impl Resources<RelaxedString> {
    /// Convert shorthand strings to raw number of cores and Bytes of memory
    pub fn normalised(&self) -> Result<Resources<f64>> {
        let requests = ResourceRequest {
            memory: parse_memory(&self.requests.memory.to_string())?,
            cpu: parse_cpu(&self.requests.cpu.to_string())?,
        };
        let limits = ResourceLimit {
            memory: parse_memory(&self.limits.memory.to_string())?,
            cpu: parse_cpu(&self.limits.cpu.to_string())?,
        };
        Ok(Resources { requests, limits })
    }
}

// For aggregation of resource use, implement addition on normalised versions
impl Add for Resources<f64> {
    type Output = Resources<f64>;

    fn add(self, rhs: Resources<f64>) -> Resources<f64> {
        let requests = ResourceRequest {
            memory: self.requests.memory + rhs.requests.memory,
            cpu: self.requests.cpu + rhs.requests.cpu,
        };
        let limits = ResourceLimit {
            memory: self.limits.memory + rhs.limits.memory,
            cpu: self.limits.cpu + rhs.limits.cpu,
        };
        Resources { requests, limits }
    }
}
impl AddAssign for Resources<f64> {
    fn add_assign(&mut self, rhs: Resources<f64>) {
        *self = self.clone() + rhs;
    }
}

impl Mul<u32> for Resources<f64> {
    type Output = Resources<f64>;

    fn mul(self, scalar: u32) -> Resources<f64> {
        let requests = ResourceRequest {
            memory: self.requests.memory * (scalar as f64),
            cpu: self.requests.cpu * (scalar as f64),
        };
        let limits = ResourceLimit {
            memory: self.limits.memory * (scalar as f64),
            cpu: self.limits.cpu * (scalar as f64),
        };
        Resources { requests, limits }
    }
}

/// Zero numericals used in computation.
/// Techncially this should be the std::num::Zero trait but it's unstable atm
impl Default for Resources<f64> {
    fn default() -> Self {
        let requests = ResourceRequest { cpu: 0.0, memory: 0.0 };
        let limits = ResourceLimit { memory: 0.0, cpu: 0.0 };
        Resources { requests, limits }
    }
}

impl Resources<f64> {
    /// Convert to gigabytes and round to two decimals
    pub fn round(&mut self) {
        self.limits.memory = (self.limits.memory * 100.0 / (1024.0 * 1024.0 * 1024.0)).round()/100.0;
        self.requests.memory = (self.requests.memory * 100.0 / (1024.0 * 1024.0 * 1024.0)).round()/100.0;
        self.limits.cpu = (self.limits.cpu * 100.0).round()/100.0;
        self.requests.cpu = (self.requests.cpu * 100.0).round()/100.0;
    }
}

impl Resources<RelaxedString> {
    // TODO: look at config for limits?
    pub fn verify(&self) -> Result<()> {
        // (We can unwrap all the values as we assume implicit called!)
        let n = self.normalised()?;
        let req = &n.requests;
        let lim = &n.limits;

        // 1.1 limits >= requests
        if req.cpu > lim.cpu {
            bail!("Requested more CPU than what was limited");
        }
        if req.memory > lim.memory {
            bail!("Requested more memory than what was limited");
        }
        // 1.2 sanity numbers (based on c5.9xlarge)
        if req.cpu > 36.0 {
            bail!("Requested more than 36 cores");
        }
        if req.memory > 72.0*1024.0*1024.0*1024.0 {
            bail!("Requested more than 72 GB of memory");
        }
        if lim.cpu > 36.0 {
            bail!("CPU limit set to more than 36 cores");
        }
        if lim.memory > 72.0*1024.0*1024.0*1024.0 {
            bail!("Memory limit set to more than 72 GB of memory");
        }
        Ok(())
    }
}



// Parse normal k8s memory resource value into floats
pub fn parse_memory(s: &str) -> Result<f64> {
    let digits = s.chars().take_while(|ch| ch.is_digit(10) || *ch == '.').collect::<String>();
    let unit = s.chars().skip_while(|ch| ch.is_digit(10) || *ch == '.').collect::<String>();
    let mut res : f64 = digits.parse()?;
    trace!("Parsed {} ({})", digits, unit);
    if unit == "Ki" {
        res *= 1024.0;
    } else if unit == "Mi" {
        res *= 1024.0*1024.0;
    } else if unit == "Gi" {
        res *= 1024.0*1024.0*1024.0;
    } else if unit == "k" {
        res *= 1000.0;
    } else if unit == "M" {
        res *= 1000.0*1000.0;
    } else if unit == "G" {
        res *= 1000.0*1000.0*1000.0;
    } else if unit != "" {
        bail!("Unknown unit {}", unit);
    }
    trace!("Returned {} bytes", res);
    Ok(res)
}

// Parse normal k8s cpu resource values into floats
// We don't allow power of two variants here
fn parse_cpu(s: &str) -> Result<f64> {
    let digits = s.chars().take_while(|ch| ch.is_digit(10) || *ch == '.').collect::<String>();
    let unit = s.chars().skip_while(|ch| ch.is_digit(10) || *ch == '.').collect::<String>();
    let mut res : f64 = digits.parse()?;

    trace!("Parsed {} ({})", digits, unit);
    if unit == "m" {
        res /= 1000.0;
    } else if unit == "k" {
        res *= 1000.0;
    } else if unit != "" {
        bail!("Unknown unit {}", unit);
    }
    trace!("Returned {} cores", res);
    Ok(res)
}
