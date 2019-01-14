//use super::traits::Verify;
use crate::structs::{Kong, Cors, BabylonAuthHeader, Authentication};
use crate::region::{KongConfig};
use std::collections::BTreeMap;
use serde::ser::{Serialize, Serializer, SerializeMap};

/// Kongfig structs
/// https://github.com/mybuilder/kongfig
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Api {
    pub name: String,
    pub plugins: Vec<ApiPlugin>,
    pub attributes: ApiAttributes,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct ApiAttributes {
    #[serde(serialize_with = "none_as_brackets")]
    pub hosts: Option<Vec<String>>,
    #[serde(serialize_with = "none_as_brackets")]
    pub uris: Option<Vec<String>>,
    #[serde(serialize_with = "none_as_brackets")]
    pub methods: Option<Vec<String>>,
    pub strip_uri: bool,
    pub preserve_host: bool,
    pub upstream_url: String,
    pub retries: u32,
    pub upstream_connect_timeout: u32,
    pub upstream_read_timeout: u32,
    pub upstream_send_timeout: u32,
    pub https_only: bool,
    pub http_if_terminated: bool,
}

/// Plugins and their configs
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct CorsPluginAttributesConfig {
    pub methods: Vec<String>,
    pub exposed_headers: Vec<String>,
    pub max_age: u32,
    pub origins: Vec<String>,
    pub headers: Vec<String>,
    pub credentials: bool,
    pub preflight_continue: bool,
}
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct CorsPluginAttributes {
    pub enabled: bool,
    pub config: CorsPluginAttributesConfig,
}

impl CorsPluginAttributes {
    fn new(cors: Cors) -> Self {
        CorsPluginAttributes {
            enabled: cors.enabled,
            config: CorsPluginAttributesConfig {
                credentials: cors.credentials,
                exposed_headers: splitter(cors.exposed_headers),
                max_age: cors.max_age.parse().unwrap(),
                methods: splitter(cors.methods),
                origins: splitter(cors.origin),
                headers: splitter(cors.headers),
                preflight_continue: cors.preflight_continue
            }
        }
    }
}

/// Serialise nil as brackets, a strange kongfig idiom
fn none_as_brackets<S, T>(t: &Option<T>, s: S) -> Result<S::Ok, S::Error>
where T: Serialize,
      S: Serializer
{
    match t {
        Some(ref value) => s.serialize_some(value),
        None            => s.serialize_map(Some(0))?.end(),
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct HeadersAndJson {
    #[serde(serialize_with = "none_as_brackets")]
    pub headers: Option<Vec<String>>,
    #[serde(serialize_with = "none_as_brackets")]
    pub json: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct ResponseTransformerPluginAttributesConfig {
    pub add: HeadersAndJson,
    pub append: HeadersAndJson,
    pub remove: HeadersAndJson,
    pub replace: HeadersAndJson,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ResponseTransformerPluginAttributes {
    pub enabled: bool,
    pub config: ResponseTransformerPluginAttributesConfig,
}

impl ResponseTransformerPluginAttributes {
    fn new(headers: BTreeMap<String, String>) -> Self {
        let mut headers_strs = Vec::new();
        for (k, v) in headers {
            headers_strs.push(format!("{}: {}", k, v));
        }
        ResponseTransformerPluginAttributes {
            enabled: true,
            config: ResponseTransformerPluginAttributesConfig {
                add: HeadersAndJson {
                    headers: Some(headers_strs),
                    json: None
                },
                append: HeadersAndJson::default(),
                remove: HeadersAndJson::default(),
                replace: HeadersAndJson::default(),
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TcpLogPluginAttributesConfig {
    pub host: String,
    pub port: u32,
    pub keepalive: u32,
    pub timeout: u32,
}
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TcpLogPluginAttributes {
    pub enabled: bool,
    pub config: TcpLogPluginAttributesConfig,
}

impl TcpLogPluginAttributes {
    fn new(host: &str, port: u32) -> Self {
        TcpLogPluginAttributes {
            enabled: true,
            config: TcpLogPluginAttributesConfig {
                host: host.into(),
                port: port.into(),
                keepalive: 60000,
                timeout: 10000,
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Oauth2PluginAttributesConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anonymous_username: Option<String>,
    pub enable_client_credentials: bool,
    pub mandatory_scope: bool,
    pub hide_credentials: bool,
    pub enable_implicit_grant: bool,
    pub global_credentials: bool,
    pub provision_key: String,
    pub enable_password_grant: bool,
    pub enable_authorization_code: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anonymous: Option<String>,
    pub token_expiration: u32,
    pub accept_http_if_already_terminated: bool,
}
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Oauth2PluginAttributes {
    pub enabled: bool,
    pub config: Oauth2PluginAttributesConfig,
}

impl Oauth2PluginAttributes {
    fn new(kong_token_expiration: u32, oauth_provision_key: &str, anonymous_consumer: Option<String>) -> Self {
        Oauth2PluginAttributes {
            enabled: true,
            config: Oauth2PluginAttributesConfig {
                anonymous: match anonymous_consumer.clone() {
                    Some(_s) => None,
                    None     => Some("".into()),
                },
                anonymous_username: anonymous_consumer.map(|_| "anonymous".into()),
                global_credentials: true,
                provision_key: oauth_provision_key.into(),
                enable_password_grant: true,
                enable_authorization_code: true,
                token_expiration: kong_token_expiration,
                ..Oauth2PluginAttributesConfig::default()
            }
        }
    }
}


#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Oauth2ExtensionPluginAttributesConfig {}
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Oauth2ExtensionPluginAttributes {
    pub enabled: bool,
    pub config: Oauth2ExtensionPluginAttributesConfig,
}

impl Default for Oauth2ExtensionPluginAttributes {
    fn default() -> Self {
        Oauth2ExtensionPluginAttributes {
            enabled: true,
            config: Oauth2ExtensionPluginAttributesConfig::default()
        }
    }
}


#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct JsonCookiesCsrfPluginAttributesConfig {
    pub csrf_field_name: String,
    pub cookie_name: String,
    pub strict: bool,
    pub csrf_header_name: String,
}
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct JsonCookiesCsrfPluginAttributes {
    pub enabled: bool,
    pub config: JsonCookiesCsrfPluginAttributesConfig,
}

impl Default for JsonCookiesCsrfPluginAttributes {
    fn default() -> Self {
        JsonCookiesCsrfPluginAttributes {
            enabled: true,
            config: JsonCookiesCsrfPluginAttributesConfig {
                cookie_name: "autologin_info".into(),
                csrf_field_name: "csrf_token".into(),
                csrf_header_name: "x-security-token".into(),
                strict: true,
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct JsonCookiesToHeadersPluginAttributesConfig {
    pub field_name: String,
    pub cookie_name: String,
}
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct JsonCookiesToHeadersPluginAttributes {
    pub enabled: bool,
    pub config: JsonCookiesToHeadersPluginAttributesConfig,
}

impl Default for JsonCookiesToHeadersPluginAttributes {
    fn default() -> Self {
        JsonCookiesToHeadersPluginAttributes {
            enabled: true,
            config: JsonCookiesToHeadersPluginAttributesConfig {
                field_name: "kong_token".into(),
                cookie_name: "autologin_token".into(),
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct BabylonAuthHeaderPluginAttributesConfig {
    pub auth_service: String,
    pub cache_timeout_sec: u32,
    pub http_timeout_msec: u32,
}
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct BabylonAuthHeaderPluginAttributes {
    pub enabled: bool,
    pub config: BabylonAuthHeaderPluginAttributesConfig,
}

impl BabylonAuthHeaderPluginAttributes {
    fn new(authheader: BabylonAuthHeader) -> Self {
        BabylonAuthHeaderPluginAttributes {
            enabled: authheader.enabled,
            config: BabylonAuthHeaderPluginAttributesConfig {
                auth_service: authheader.auth_service,
                cache_timeout_sec: authheader.cache_timeout_sec,
                http_timeout_msec: authheader.http_timeout_msec,
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct CorrelationIdPluginAttributesConfig {
    pub echo_downstream: bool,
    pub header_name: String,
    pub generator: String,
}
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct CorrelationIdPluginAttributes {
    pub enabled: bool,
    pub config: CorrelationIdPluginAttributesConfig,
}

impl Default for CorrelationIdPluginAttributes {
    fn default() -> Self {
        CorrelationIdPluginAttributes {
            enabled: true,
            config: CorrelationIdPluginAttributesConfig {
                echo_downstream: true,
                header_name: "babylon-request-id".into(),
                generator: "uuid".into(),
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "name")]
#[serde(rename_all = "kebab-case")]
pub enum ApiPlugin {
    TcpLog(PluginBase<TcpLogPluginAttributes>),
    Oauth2(PluginBase<Oauth2PluginAttributes>),
    Oauth2Extension(PluginBase<Oauth2ExtensionPluginAttributes>),
    Cors(PluginBase<CorsPluginAttributes>),
    CorrelationId(PluginBase<CorrelationIdPluginAttributes>),
    BabylonAuthHeader(PluginBase<BabylonAuthHeaderPluginAttributes>),
    JsonCookiesToHeaders(PluginBase<JsonCookiesToHeadersPluginAttributes>),
    JsonCookiesCsrf(PluginBase<JsonCookiesCsrfPluginAttributes>),
    ResponseTransformer(PluginBase<ResponseTransformerPluginAttributes>),
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct PluginBase<T> {
    #[serde(skip_serializing_if = "Ensure::is_present")]
    pub ensure: Ensure,
    pub attributes: T,
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Ensure {
    Present,
    Removed,
}

impl Default for Ensure {
    fn default() -> Self { Ensure::Present }
}

impl Ensure {
    fn is_present(x: &Ensure) -> bool {
        x == &Ensure::Present
    }
}

fn splitter(value: String) -> Vec<String> {
    value.split(',')
        .map(|h| h.trim())
        .map(String::from)
        .collect()
}

pub fn kongfig_apis(from: BTreeMap<String, Kong>, config: KongConfig) -> Vec<Api> {
    let mut apis = Vec::new();
    for (k, v) in from.clone() {
        let mut plugins = Vec::new();

        // Prepare plugins

        // Always: CorrelationId
        plugins.push(ApiPlugin::CorrelationId(PluginBase::<CorrelationIdPluginAttributes>::default()));

        // If globally enabled: TCP Logging
        if config.tcp_log.enabled {
            plugins.push(ApiPlugin::TcpLog(PluginBase::<TcpLogPluginAttributes> {
                ensure: Ensure::default(),
                attributes: TcpLogPluginAttributes::new(&config.tcp_log.host, config.tcp_log.port.parse().unwrap())
            }));
        }

        // If enabled: Oauth2 and extension
        let en = match v.auth {
            Authentication::OAuth2 => Ensure::default(),
            _ => Ensure::Removed,
        };

        plugins.push(ApiPlugin::Oauth2(PluginBase::<Oauth2PluginAttributes> {
            ensure: en,
            attributes: Oauth2PluginAttributes::new(
                config.kong_token_expiration,
                &config.oauth_provision_key,
                v.oauth2_anonymous)
        }));

        if v.oauth2_extension_plugin.unwrap_or(false) {
            plugins.push(ApiPlugin::Oauth2Extension(PluginBase::<Oauth2ExtensionPluginAttributes>::default()));
        }

        // If enabled: babylon-auth-header
        if let Some(babylon_auth_header) = v.babylon_auth_header {
            plugins.push(ApiPlugin::BabylonAuthHeader(PluginBase::<BabylonAuthHeaderPluginAttributes> {
                ensure: Ensure::default(),
                attributes: BabylonAuthHeaderPluginAttributes::new(babylon_auth_header)
            }));
        }

        // If enabled: CORS
        if let Some(cors) = v.cors {
            plugins.push(ApiPlugin::Cors(PluginBase::<CorsPluginAttributes> {
                ensure: Ensure::default(),
                attributes: CorsPluginAttributes::new(cors)
            }));
        }

        // If enabled: ResponseTransformer to add headers
        if let Some(add_headers) = v.add_headers {
            plugins.push(ApiPlugin::ResponseTransformer(PluginBase::<ResponseTransformerPluginAttributes> {
                ensure: Ensure::default(),
                attributes: ResponseTransformerPluginAttributes::new(add_headers)
            }));
        }

        // If enabled: JsonCookies and JsonCookiesCsrf
        if v.cookie_auth {
            plugins.push(ApiPlugin::JsonCookiesToHeaders(PluginBase::<JsonCookiesToHeadersPluginAttributes>::default()));
        }

        if v.cookie_auth_csrf {
            plugins.push(ApiPlugin::JsonCookiesCsrf(PluginBase::<JsonCookiesCsrfPluginAttributes>::default()));
        }

        // Create the main API object
        apis.push(Api {
            name: k.to_string(),
            plugins: plugins,
            attributes: ApiAttributes {
                hosts: v.hosts.map(splitter),
                uris: v.uris.map(|s| vec![s]),
                preserve_host: true,
                strip_uri: v.strip_uri,
                upstream_connect_timeout: v.upstream_connect_timeout.unwrap_or(30000),
                upstream_read_timeout: v.upstream_read_timeout.unwrap_or(30000),
                upstream_send_timeout: v.upstream_send_timeout.unwrap_or(30000),
                upstream_url: v.upstream_url,
                ..Default::default()
            }
        });
    }
    apis
}

pub fn kongfig_consumers(k: KongConfig) -> Vec<Consumer> {
    let mut consumers: Vec<Consumer> = k.consumers.into_iter().map(|(k,v)| {
        Consumer {
            username: k.to_string(),
            acls: vec![],
            credentials: vec![ConsumerCredentials {
                name: "oauth2".into(),
                attributes: ConsumerCredentialsAttributes {
                    name: v.username,
                    client_id: v.oauth_client_id,
                    client_secret: v.oauth_client_secret,
                    redirect_uri: vec!["http://example.com/unused".into()]
                }
            }],
        }
    }).collect();

    // Add the anonymous customer as well
    consumers.push(Consumer {
        username: "anonymous".into(),
        acls: vec![],
        credentials: vec![]
    });

    consumers
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub username: String,
    pub acls: Vec<String>,
    pub credentials: Vec<ConsumerCredentials>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ConsumerCredentials {
    pub name: String,
    pub attributes: ConsumerCredentialsAttributes,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ConsumerCredentialsAttributes {
    pub client_id: String,
    pub redirect_uri: Vec<String>,
    pub name: String,
    pub client_secret: String,
}


/// Not used yet
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Plugin {}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Upstream {}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Certificate {}
