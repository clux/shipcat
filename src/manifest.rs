use serde_yaml;

use std::io::prelude::*;
use std::fs::File;
use std::env;
use std::path::{PathBuf, Path};
use std::collections::{HashMap, BTreeMap};

use super::Result;
use super::vault::Vault;

// k8s related structs

#[derive(Serialize, Deserialize, Clone)]
pub struct ResourceRequest {
    /// CPU request string
    cpu: String,
    /// Memory request string
    memory: String,
    // TODO: ephemeral-storage + extended-resources
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ResourceLimit {
    /// CPU limit string
    cpu: String,
    /// Memory limit string
    memory: String,
    // TODO: ephemeral-storage + extended-resources
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Resources {
    /// Resource requests for k8s
    pub requests: Option<ResourceRequest>,
    /// Resource limits for k8s
    pub limits: Option<ResourceLimit>,
}


#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Replicas {
    /// Minimum replicas for k8s deployment
    pub min: u32,
    /// Maximum replicas for k8s deployment
    pub max: u32,
}


#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ConfigMount {
    /// Name of file to template
    pub name: String,
    /// Name of file as used in code
    pub dest: String,
    /// Volume directory in docker
    pub volume: String,
}

// misc structs

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


//#[derive(Serialize, Clone, Default, Debug)]
//pub struct PortMap {
//    /// Host port
//    pub host: u32,
//    /// Target port
//    pub target: u32,
//}

/// Main manifest, serializable from shipcat.yml
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Manifest {
    /// Name of the main component
    pub name: String,

    // Kubernetes specific flags

    /// Resource limits and requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Resources>,
    /// Replication limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replicas: Option<Replicas>,
    /// Environment variables to inject
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<BTreeMap<String, String>>,
    /// Environment files to mount
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub config: Vec<ConfigMount>,
    /// Ports to expose
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<u32>,

    // TODO: boot time -> minReadySeconds


    /// Prometheus metric options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prometheus: Option<Prometheus>,
//prometheus:
//  enabled: true
//  path: /metrics
    /// Dashboards to generate
    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub dashboards: BTreeMap<String, Dashboard>,
//dashboards:
//  auth-python:
//    rows:
//      - users-connected
//      - conversation-length

    // TODO: health/die, secrets/vault, logging alerts
//vault:
//  path: /blah/woot
//logging:
//  alerts:
//    error-rate-5xx:
//      type: median
//      threshold: 2
//      status-code: 500
//preStopHookPath: /die
// newrelic options? we generate the newrelic.ini from a vault secret + manifest.name

    // Internal path of this manifest
    #[serde(skip_serializing, skip_deserializing)]
    _location: String,

//    // Parsed port map of this manifest
//    #[serde(skip_serializing, skip_deserializing)]
//    pub _portmaps: Vec<PortMap>
}


impl Manifest {
    pub fn new(name: &str, location: PathBuf) -> Manifest {
        Manifest {
            name: name.into(),
            _location: location.to_string_lossy().into(),
            ..Default::default()
        }
    }
    /// Read a manifest file in an arbitrary path
    pub fn read_from(pwd: &PathBuf) -> Result<Manifest> {
        let mpath = pwd.join("shipcat.yml");
        trace!("Using manifest in {}", mpath.display());
        let mut f = File::open(&mpath)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        let mut res: Manifest = serde_yaml::from_str(&data)?;
        // store the location internally (not serialized to disk)
        res._location = mpath.to_string_lossy().into();
        Ok(res)
    }

    /// Read a manifest file in PWD
    pub fn read() -> Result<Manifest> {
        Ok(Manifest::read_from(&Path::new(".").to_path_buf())?)
    }

    /// Populate implicit defaults from config file
    ///
    /// Currently assume defaults live in environment directory
    fn implicits(&mut self, env: &str) -> Result<()> {
        let cfg_dir = env::current_dir()?.join(env); // TODO: generalize
        let def_pth = cfg_dir.join("shipcat.yml");
        let mut f = File::open(&def_pth)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        let mf: Manifest = serde_yaml::from_str(&data)?;

        if self.resources.is_none() {
            self.resources = mf.resources.clone();
        }
        if let Some(ref mut res) = self.resources {
            if res.limits.is_none() {
                res.limits = mf.resources.clone().unwrap().limits;
            }
            if res.requests.is_none() {
                res.requests = mf.resources.clone().unwrap().requests;
            }
            // for now: if limits or requests are specified, you have to fill in both CPU and memory
        }
        if self.replicas.is_none() {
            self.replicas = mf.replicas;
        }
        // only using target ports now, disabling this now
        //for s in &self.ports {
        //    self._portmaps.push(parse_ports(s)?);
        //}
        Ok(())
    }

    // Populate placeholder fields with secrets from vault
    fn secrets(&mut self, client: &mut Vault, service: &str, env: &str) -> Result<()> {
        let envmap: HashMap<&str, &str> =[
            ("dev", "development"), // dev env uses vault secrets in development
        ].iter().cloned().collect();

        if let Some(mut envs) = self.env.clone() {
            // iterate over evar key values and find the ones we need
            for (key, value) in envs.iter_mut() {
                if value == &"IN_VAULT" {
                    let full_key = format!("{}/{}", service, key);
                    let secret = client.read(envmap.get(env).unwrap(), &full_key)?;
                    *value = secret;
                }
            }
            self.env = Some(envs); // overwrite env key with our populated one
        }
        Ok(())
    }

    // Return a completed (read, filled in, and populate secrets) manifest
    pub fn completed(env: &str, service: &str, client: &mut Vault) -> Result<Manifest> {
        let pth = Path::new(".").join(env).join(service);
        let mut mf = Manifest::read_from(&pth)?;
        mf.implicits(env)?;
        mf.secrets(client, service, env)?;

        Ok(mf)
    }

    /// Update the manifest file in the current folder
    pub fn write(&self) -> Result<()> {
        let encoded = serde_yaml::to_string(self)?;
        trace!("Writing manifest in {}", self._location);
        let mut f = File::create(&self._location)?;
        write!(f, "{}\n", encoded)?;
        debug!("Wrote manifest in {}: \n{}", self._location, encoded);
        Ok(())
    }

    /// Verify assumptions about manifest
    ///
    /// Assumes the manifest has been populated with `implicits`
    pub fn verify(&self) -> Result<()> {
        if self.name == "" {
            bail!("Name cannot be empty")
        }
        // 1. Verify resources
        // (We can unwrap all the values as we assume implicit called!)
        let req = self.resources.clone().unwrap().requests.unwrap().clone();
        let lim = self.resources.clone().unwrap().limits.unwrap().clone();
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
        if req_memory > 10*1024*1024*1024 {
            bail!("Requested more than 10 GB of memory");
        }

        // 2. Ports restrictions? currently parse only


        // X. TODO: other keys

        Ok(())
    }
}

// Parse normal docker style host:target port opening
// disabled for now - only parsing target port vector
/*fn parse_ports(s: &str) -> Result<PortMap> {
    let split: Vec<&str> = s.split(':').collect();
    if split.len() != 2 {
        bail!("Port listing {} not in the form of host:target", s);
    }
    let host = split[0].parse().map_err(|e| {
        warn!("Invalid host port {} could not be parsed", split[0]);
        e
    })?;
    let target = split[1].parse().map_err(|e| {
        warn!("Invalid target port {} could not be parsed", split[0]);
        e
    })?;
    Ok(PortMap{ host, target })
}*/

// Parse normal k8s memory resource value into integers
fn parse_memory(s: &str) -> Result<u64> {
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
fn parse_cpu(s: &str) -> Result<f64> {
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

pub fn validate() -> Result<()> {
    let mut mf = Manifest::read()?;
    mf.implicits("dev")?; // TODO: this is going to fail anyway now
    mf.verify()
}

pub fn init() -> Result<()> {
    let pwd = env::current_dir()?;
    let last_comp = pwd.components().last().unwrap(); // std::path::Component
    let dirname = last_comp.as_os_str().to_str().unwrap();

    let mf = Manifest::new(dirname, pwd.join("shipcat.yml"));
    mf.write()
}
