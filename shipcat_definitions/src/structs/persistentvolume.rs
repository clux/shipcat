use super::Result;
use super::resources::parse_memory;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct PersistentVolume {
    pub name: String,
    pub claim: String,
    pub storageClass: String,
    pub accessMode: String,
    pub size: String,
}

impl PersistentVolume {
    pub fn verify(&self) -> Result<()> {
        let size = parse_memory(&self.size)?;
        if size > 100.0*1024.0*1024.0*1024.0 {
            bail!("Memory size set to more than 100 GB of persistent memory")
        }
        Ok(())
    }
}
