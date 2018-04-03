use regex::Regex;

use super::traits::Verify;
use super::{Result, Config};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct InitContainer {
    pub name: String,
    pub image: String,
    pub command: Vec<String>,
}

impl Verify for InitContainer {
    // only verify syntax
    fn verify(&self, _: &Config) -> Result<()> {
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
