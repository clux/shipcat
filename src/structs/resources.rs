use super::traits::Verify;
use super::{Result, Config};

/// Kubernetes resource requests
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ResourceRequest {
    /// CPU request string
    pub cpu: String,
    /// Memory request string
    pub memory: String,
    // TODO: ephemeral-storage + extended-resources
}

/// Kubernetes resource limits
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ResourceLimit {
    /// CPU limit string
    pub cpu: String,
    /// Memory limit string
    pub memory: String,
    // TODO: ephemeral-storage + extended-resources
}

/// Kubernetes resources
///
/// This can be inlined straight into a container spec at the moment
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Resources {
    /// Resource requests for k8s
    pub requests: Option<ResourceRequest>,
    /// Resource limits for k8s
    pub limits: Option<ResourceLimit>,
}


impl Verify for Resources {
    // TODO: look at config for limits
    fn verify(&self, _: &Config) -> Result<()> {
        // (We can unwrap all the values as we assume implicit called!)
        let req = self.clone().requests.unwrap();
        let lim = self.clone().limits.unwrap();

        let req_memory = parse_memory(&req.memory)?;
        let lim_memory = parse_memory(&lim.memory)?;
        let req_cpu = parse_cpu(&req.cpu)?;
        let lim_cpu = parse_cpu(&lim.cpu)?;

        // 1.1 limits >= requests
        if req_cpu > lim_cpu {
            bail!("Requested more CPU than what was limited");
        }
        if req_memory > lim_memory {
            bail!("Requested more memory than what was limited");
        }
        // 1.2 sanity numbers
        if req_cpu > 10.0 {
            bail!("Requested more than 10 cores");
        }
        if req_memory > 20.0*1024.0*1024.0*1024.0 {
            bail!("Requested more than 20 GB of memory");
        }
        if lim_cpu > 20.0 {
            bail!("CPU limit set to more than 20 cores");
        }
        if lim_memory > 30.0*1024.0*1024.0*1024.0 {
            bail!("Memory limit set to more than 30 GB of memory");
        }
        Ok(())
    }
}



// Parse normal k8s memory resource value into floats
fn parse_memory(s: &str) -> Result<f64> {
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
