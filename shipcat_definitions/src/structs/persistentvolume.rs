use super::Result;
use super::resources::parse_memory;

/// K8s Access modes for PVCs
///
/// See [K8s access mode docs](https://kubernetes.io/docs/concepts/storage/persistent-volumes/#access-modes).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum VolumeAccessMode {
    ReadWriteOnce,
    ReadOnlyMany,
    ReadWriteMany,
}

impl Default for VolumeAccessMode {
    fn default() -> Self {
        Self::ReadWriteOnce // most supported mode
    }
}

/// A kubernetes Persistent Volume Claim
///
/// See [K8s persistent volume docs](https://kubernetes.io/docs/concepts/storage/persistent-volumes/)-.
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct PersistentVolume {
    pub name: String,
    pub mountPath: String,
    pub size: String,
    #[serde(default)]
    pub accessMode: VolumeAccessMode,
}

impl PersistentVolume {
    pub fn verify(&self) -> Result<()> {
        let size = parse_memory(&self.size)?;
        // sanity number; 16TB via https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ebs-volume-types.html
        if size > 16.0*1024.0*1024.0*1024.0*1024.0 {
            bail!("Persistent Volume request more than 16 TB")
        }
        if !self.mountPath.starts_with('/') {
            bail!("Mount path '{}' must start with a slash", self.mountPath);
        }
        if self.mountPath.ends_with('/') {
            bail!("Mount path '{}' must not end with a slash", self.mountPath);
        }
        Ok(())
    }
}
