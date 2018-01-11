use serde_yaml;

use std::io::prelude::*;
use std::fs::File;
use std::path::{PathBuf, Path};
use std::collections::BTreeMap;

use super::BabylResult;

#[derive(Serialize, Deserialize, Clone)]
pub struct ResourceRequest {
    /// CPU request string
    cpu: String,
    /// Memory request string
    memory: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ResourceLimit {
    /// CPU limit string
    cpu: String,
    /// Memory limit string
    memory: String,
}

impl Default for ResourceLimit {
    fn default() -> Self {
        ResourceLimit {
            cpu: "800m".into(),
            memory: "1024Mi".into(),
        }
    }
}
impl Default for ResourceRequest {
    fn default() -> Self {
        ResourceRequest {
            cpu: "200m".into(),
            memory: "512Mi".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Resources {
    /// Resource requests for k8s
    pub requests: ResourceRequest,
    /// Resource limits for k8s
    pub limits: ResourceLimit,
}


#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Dashboard {
    /// Metric strings to track
    pub rows: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Prometheus {
    /// Whether to poll
    pub enabled: bool,
    /// Path to poll
    pub path: String,
    // TODO: Maybe include names of metrics?
}


#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Manifest {
    /// Name of the main component
    pub name: String,

    /// Default environment to build in
    pub resources: Resources,

    /// Prometheus metric options
    pub prometheus: Prometheus,

    /// Dashboards to generate
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub dashboards: BTreeMap<String, Dashboard>,

    /// Internal path of this manifest
    #[serde(skip_serializing, skip_deserializing)]
    location: String,
}

impl Manifest {
    pub fn new(name: &str, location: PathBuf) -> Manifest {
        Manifest {
            name: name.into(),
            location: location.to_string_lossy().into(),
            ..Default::default()
        }
    }
    /// Read a manifest file in an arbitrary path
    pub fn read_from(pwd: &PathBuf) -> BabylResult<Manifest> {
        let mpath = pwd.join("babyl.yaml");
        trace!("Using manifest in {}", mpath.display());
        let mut f = File::open(&mpath)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        let mut res: Manifest = serde_yaml::from_str(&data)?;
        // store the location internally (not serialized to disk)
        res.location = mpath.to_string_lossy().into();
        Ok(res)
    }

    /// Read a manifest file in PWD
    pub fn read() -> BabylResult<Manifest> {
        Ok(Manifest::read_from(&Path::new(".").to_path_buf())?)
    }

    /// Update the manifest file in the current folder
    pub fn write(&self) -> BabylResult<()> {
        let encoded = serde_yaml::to_string(self)?;
        trace!("Writing manifest in {}", self.location);
        let mut f = File::create(&self.location)?;
        write!(f, "{}\n", encoded)?;
        debug!("Wrote manifest in {}: \n{}", self.location, encoded);
        Ok(())
    }

    /// Verify assumptions about manifest
    pub fn verify(&self) -> BabylResult<()> {
        if self.name == "" {
            bail!("Name cannot be empty")
        }
        // 1. Verify resources
        let req_cpu = parse_cpu(&self.resources.requests.cpu)?;
        let lim_cpu = parse_cpu(&self.resources.limits.cpu)?;
        let req_memory = parse_memory(&self.resources.requests.memory)?;
        let lim_memory = parse_memory(&self.resources.limits.memory)?;
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
        if req_memory > 10*1024*1024*1024 {
            bail!("Requested more than 10 GB of memory");
        }

        // 2. TODO: other keys

        Ok(())
    }

}

// Parse normal k8s memory resource value into integers
fn parse_memory(s: &str) -> BabylResult<u64> {
    let digits = s.chars().take_while(|ch| ch.is_digit(10)).collect::<String>();
    let unit = s.chars().skip_while(|ch| ch.is_digit(10)).collect::<String>();
    let mut res : u64 = digits.parse()?;
    trace!("Parsed {} ({})", digits, unit);
    if unit == "Ki" {
        res *= 1024;
    } else if unit == "Mi" {
        res *= 1024*1024;
    } else if unit == "Gi" {
        res *= 1024*1024*1024;
    } else if unit == "k" {
        res *= 1000;
    } else if unit == "M" {
        res *= 1000*1000;
    } else if unit == "G" {
        res *= 1000*1000*1000;
    } else if unit != "" {
        bail!("Unknown unit {}", unit);
    }
    trace!("Returned {} bytes", res);
    Ok(res)
}

// Parse normal k8s cpu resource values into floats
// We don't allow power of two variants here
fn parse_cpu(s: &str) -> BabylResult<f64> {
    let digits = s.chars().take_while(|ch| ch.is_digit(10)).collect::<String>();
    let unit = s.chars().skip_while(|ch| ch.is_digit(10)).collect::<String>();
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
