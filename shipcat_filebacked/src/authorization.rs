use merge::Merge;

use shipcat_definitions::structs::Authorization;

use super::Result;
use super::util::{Build};

#[derive(Deserialize, Default, Merge, Clone)]
pub struct AuthorizationSource {
    pub allowed_audiences: Option<Vec<String>>,
    pub allow_anonymous: Option<bool>,
    pub allow_invalid_tokens: Option<bool>,

    pub required_scopes: Option<Vec<String>>,
    pub allow_cookies: Option<bool>,

    pub enable_cookie_refresh: Option<bool>,
    pub refresh_auth_service: Option<String>,
    pub refresh_body_refresh_token_key: Option<String>,
    pub refresh_cookie_domain: Option<String>,
    pub refresh_max_age_sec: Option<u32>,
    pub refresh_http_timeout_msec: Option<u32>,
    pub refresh_renew_before_expiry_sec: Option<u32>,
}

impl Build<Authorization, ()> for AuthorizationSource {
    fn build(self, _params: &()) -> Result<Authorization> {
        let allowed_audiences = self.allowed_audiences.unwrap_or_default();
        if allowed_audiences.is_empty() {
            bail!("allowed_audiences must contain at least one audience");
        }
        let allow_anonymous = self.allow_anonymous.unwrap_or_default();
        let allow_invalid_tokens = self.allow_invalid_tokens.unwrap_or_default();
        if allow_invalid_tokens && !allow_anonymous {
            bail!("allow_invalid_tokens requires allow_anonymous");
        }

        let enable_cookie_refresh = self.enable_cookie_refresh.unwrap_or(false);
        if enable_cookie_refresh && (self.refresh_auth_service.is_none() || self.refresh_body_refresh_token_key.is_none()){
            bail!("enable_cookie_refresh requires refresh_auth_service and refresh_body_refresh_token_key");
        }

        Ok(Authorization {
            allowed_audiences,
            allow_anonymous,
            allow_invalid_tokens,
            enable_cookie_refresh,
            required_scopes: self.required_scopes.unwrap_or_default(),
            allow_cookies: self.allow_cookies.unwrap_or_default(),
            refresh_auth_service: self.refresh_auth_service,
            refresh_body_refresh_token_key: self.refresh_body_refresh_token_key,
            refresh_cookie_domain: self.refresh_cookie_domain,
            refresh_max_age_sec: self.refresh_max_age_sec,
            refresh_http_timeout_msec: self.refresh_http_timeout_msec,
            refresh_renew_before_expiry_sec: self.refresh_renew_before_expiry_sec,
        })
    }
}
