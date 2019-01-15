use super::{Result, Region};
use std::ops::Not;
use std::collections::BTreeMap;

/// Kong setup for a service
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Kong {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub upstream_url: String,
    #[serde(default, skip_serializing)]
    pub unauthenticated: bool,
    #[serde(default, skip_serializing_if = "Not::not")]
    pub internal: bool,
    #[serde(default, skip_serializing_if = "Not::not")]
    pub publiclyAccessible: bool,
    #[serde(default, skip_serializing_if = "Not::not")]
    pub cookie_auth: bool,
    #[serde(default, skip_serializing_if = "Not::not")]
    pub cookie_auth_csrf: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uris: Option<String>,
    /// Value filled by manifest
    #[serde(default, skip_serializing)]
    pub host: Option<String>,
    /// Full value sent to Kong
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosts: Option<String>,
    #[serde(default)]
    pub auth: Authentication,
    #[serde(default)]
    pub strip_uri: bool,
    #[serde(default = "preserve_host_default")]
    pub preserve_host: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cors: Option<Cors>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_internal_ips: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub babylon_auth_header: Option<BabylonAuthHeader>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth2_anonymous: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth2_extension_plugin: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_connect_timeout: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_send_timeout: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_read_timeout: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add_headers: Option<BTreeMap<String, String>>,
}


impl Kong {
    pub fn implicits(&mut self, svc: String, reg: Region, tophosts: Vec<String>) {
        self.name = svc;
        if self.unauthenticated {
            self.auth = Authentication::None;
        }
        // Generate upstream_url for an in-kubernetes service
        if self.upstream_url.is_empty() {
          self.upstream_url = format!("http://{}.{}.svc.cluster.local", self.name, reg.namespace);
        }

        if tophosts.is_empty() {
            // If the `host` field is defined, generate a `hosts` field based on the environment
            if let Some(h) = self.host.clone() {
                self.hosts = Some(format!("{}{}", h, reg.kong.base_url));
            }
        } else {
            self.hosts = Some(tophosts.join(","));
        }
    }

    /// Merge in fields from an override, if they're set
    pub fn merge(&mut self, other: Kong) {
        if let Some(cors) = other.cors {
            self.cors = Some(cors.clone());
        }
        if ! other.additional_internal_ips.is_empty() {
            self.additional_internal_ips = other.additional_internal_ips.clone();
        }
    }
}

fn preserve_host_default() -> bool { true }

/// Cors plugin data
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Cors {
    pub credentials: bool,
    pub enabled: bool,
    pub exposed_headers: String,
    pub headers: String,
    pub max_age: String,
    pub methods: String,
    pub origin: String,
    pub preflight_continue: bool
}

/// Babylon Auth Header plugin data
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct BabylonAuthHeader {
    pub auth_service: String,
    pub cache_timeout_sec: u32,
    pub enabled: bool,
    pub http_timeout_msec: u32,
}

impl Kong {
    pub fn verify(&self) -> Result<()> {
        if self.uris.is_none() && self.host.is_none() {
            bail!("One of `uris` or `host` needs to be defined for Kong");
        }
        if self.uris.is_some() && self.host.is_some() {
            bail!("Only one of `uris` or `host` needs to be defined for Kong");
        }
        match self.auth {
            Authentication::OAuth2 => {},
            Authentication::None => {
                if let Some(_) = self.oauth2_anonymous {
                    bail!("`oauth2_anonymous` not supported when Kong `auth` is `none`");
                }
                if let Some(true) = self.oauth2_extension_plugin {
                    bail!("`oauth2_extension_plugin` not supported when Kong `auth` is `none`");
                }
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum Authentication {
    None,
    // Not o-auth2
    #[serde(rename = "oauth2")]
    OAuth2,
}

impl Default for Authentication {
    fn default() -> Self {
        Authentication::OAuth2
    }
}
