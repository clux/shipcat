use std::path::Path;
use super::traits::Verify;
use super::{Result, Config, Manifest};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ChildComponent {
    pub name: String,
}

impl Verify for ChildComponent {
    fn verify(&self, conf: &Config) -> Result<()> {
        // self.name must exist in services/
        let dpth = Path::new(".").join("services").join(self.name.clone());
        if !dpth.is_dir() {
            bail!("Service {} does not exist in services/", self.name);
        }
        let child = Manifest::basic(&self.name, conf, None)?;
        if !child.children.is_empty() {
            bail!("Child component {} cannot have children", self.name);
        }

        // TODO: dependent service must use same image name
        Ok(())
    }
}
