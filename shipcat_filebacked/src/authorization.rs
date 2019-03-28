use merge::Merge;

use shipcat_definitions::structs::Authorization;

use super::Result;

#[derive(Deserialize, Default, Merge, Clone)]
pub struct AuthorizationSource {
    pub enabled: Option<bool>,
    pub allowed_audiences: Option<Vec<String>>,
    pub allow_anonymous: Option<bool>,
    pub remove_invalid_tokens: Option<bool>,

    pub required_scopes: Option<Vec<String>>,
    pub allow_cookies: Option<bool>,
}

impl AuthorizationSource {
    pub fn build(self) -> Result<Option<Authorization>> {
        if !self.enabled.unwrap_or_default() {
            return Ok(None);
        }
        let allowed_audiences = self.allowed_audiences.unwrap_or_default();
        if allowed_audiences.is_empty() {
            bail!("allowed_audiences must contain at least one audience");
        }
        Ok(Some(Authorization {
            allowed_audiences,
            allow_anonymous: self.allow_anonymous.unwrap_or_default(),
            remove_invalid_tokens: self.remove_invalid_tokens.unwrap_or(true),
            required_scopes: self.required_scopes.unwrap_or_default(),
            allow_cookies: self.allow_cookies.unwrap_or_default(),
        }))
    }
}
