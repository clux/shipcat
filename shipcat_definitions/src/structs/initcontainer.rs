use regex::Regex;
use super::Result;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct InitContainer {
    pub name: String,
    pub image: String,
    pub command: Vec<String>,
}

impl InitContainer {
    /// Verify syntax
    pub fn verify(&self) -> Result<()> {
        let re = Regex::new(r"(?:[a-z]+/)?([a-z]+)(?::[0-9]+)?").unwrap();
        if !re.is_match(&self.image) {
            bail!("The init container {} does not seem to match a valid image registry", self.name);
        }
        if self.command.is_empty() {
            bail!("A command must be specified for the init container {}", self.name);
        }
        Ok(())
    }
}
