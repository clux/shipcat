use super::{Result, Region};
use std::ops::Not;
use std::collections::BTreeMap;

/// Kong setup for a service
#[derive(Serialize, Deserialize, Clone, Default)]
#[cfg_attr(filesystem, serde(deny_unknown_fields))]
pub struct Kong {
    /// Auto-populated name of service
    ///
    /// Set internally, and used for defaults (discards value in manifest)
    /// But overrideable for `extra_apis` in `Region::kong`.
    #[serde(default)]
    pub name: String,

    /// The base target URL that points to your API server.
    ///
    /// This URL will be used for proxying requests. For example: https://example.com.
    ///
    /// Normal kubernetes value is: raftcat.svc.cluster.local
    /// If left blank, this value will be generated with the service name instead of raftcat.
    #[serde(default)]
    pub upstream_url: String,

    /// Whether the oauth2 plugin will be applied or not to this api
    #[serde(default, skip_serializing)]
    pub unauthenticated: bool,

    /// Whether or not to apply the ip whitelisting (?)
    #[serde(default, skip_serializing_if = "Not::not")]
    pub internal: bool,

    /// Marker for gate to let external traffic in or not
    #[serde(default, skip_serializing_if = "Not::not")]
    pub publiclyAccessible: bool,

    /// Whether to allow cookie based authentication for front-end applications
    #[serde(default, skip_serializing_if = "Not::not")]
    pub cookie_auth: bool,

    /// Whether or not to CSRF for cookie auths (mattmalones plugin)
    #[serde(default, skip_serializing_if = "Not::not")]
    pub cookie_auth_csrf: bool,


    /// Simple path based routing
    ///
    /// E.g. /raftcat
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uris: Option<String>,

    /// A comma-separated list of domain names that point to your API.
    ///
    /// For example: example.com. At least one of hosts, uris, or methods should be specified
    ///
    /// This is the full value sent to kong.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosts: Option<String>,

    /// Convenience way to specify a single host rather than using hosts above
    ///
    /// Will create a single hosts output prefixed with the kong base url.
    /// Mutually exclusive with `hosts`.
    #[serde(default, skip_serializing)]
    pub host: Option<String>,

    /// Authentication type
    #[serde(default)]
    pub auth: Authentication,

    /// When matching an API via one of the uris prefixes, strip that matching prefix from the upstream URI to be requested.
    ///
    /// false => application has to listen on the `uris` parameter (e.g. /raftcat)
    /// true => application has to listen on `/`, but use prefix agnostic urls everywhere.
    #[serde(default)]
    pub strip_uri: bool,

    /// Preserves host headers to backend service
    ///
    /// When matching an API via one of the hosts domain names, make sure the request
    /// Host header is forwarded to the upstream service. Kong's default is false,
    /// meaning the upstream Host header will be extracted from the configured upstream_url.
    ///
    /// Shipcat assumes a default of true, as the normal use case is to have this enabled.
    #[serde(default = "preserve_host_default")]
    pub preserve_host: bool,

    /// Configuration parameters for Cross Origin Resource Sharing plugin
    ///
    /// When set, the plugin is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cors: Option<Cors>,

    /// When internal is set to true, also add allow these ips through
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_internal_ips: Vec<String>,

    /// Babyln plugin (Vincent's) for propagating a core-ruby auth header.
    ///
    /// Compatibility layer for old-style core-ruby authorization headers.
    /// Deprecated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub babylon_auth_header: Option<BabylonAuthHeader>,

    /// Whether or not to allow anonymous users to go through an authenticated API
    ///
    /// Has to match the ID of the anonymous user in the kong configuration.
    /// See Region::kong::anonymous_consumers
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth2_anonymous: Option<String>,

    /// Whether or not to use the oauth2 extension plugin for this api.
    ///
    /// TODO: can probably be deprecated?
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth2_extension_plugin: Option<bool>,

    /// The timeout in milliseconds for establishing a connection to your upstream service. Defaults to 6000
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_connect_timeout: Option<u32>,

    /// The timeout in milliseconds between two successive write operations for transmitting a request to your upstream service Defaults to 60000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_send_timeout: Option<u32>,

    /// The timeout in milliseconds between two successive read operations for transmitting a request to your upstream service Defaults to 60000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_read_timeout: Option<u32>,

    /// Extra headers to append to the response from kong after reverse proxying
    ///
    /// I.e. the application will receive these extra headers.
    ///
    /// ```yaml
    /// add_headers:
    ///   Cache-Control: no-cache, no-store
    ///   Strict-Transport-Security: max-age=31536000; includeSubDomains; preload;
    ///   X-Content-Type-Options: nosniff
    ///   X-Frame-Options: SAMEORIGIN
    ///   X-XSS-Protection: 1; mode=block
    /// ```
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
#[cfg_attr(filesystem, serde(deny_unknown_fields))]
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
#[cfg_attr(filesystem, serde(deny_unknown_fields))]
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
            Authentication::Jwt => {
                if let Some(true) = self.oauth2_extension_plugin {
                    bail!("`oauth2_extension_plugin` not supported when Kong `auth` is `jwt`");
                }
            }
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
    Jwt,
}

impl Default for Authentication {
    fn default() -> Self {
        Authentication::OAuth2
    }
}
