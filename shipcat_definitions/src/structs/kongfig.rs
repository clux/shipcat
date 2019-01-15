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
pub struct CorsPluginConfig {
    pub methods: Vec<String>,
    pub exposed_headers: Vec<String>,
    pub max_age: u32,
    pub origins: Vec<String>,
    pub headers: Vec<String>,
    pub credentials: bool,
    pub preflight_continue: bool,
}

impl CorsPluginConfig {
    fn new(cors: Cors) -> Self {
        CorsPluginConfig {
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
pub struct ResponseTransformerPluginConfig {
    pub add: HeadersAndJson,
    pub append: HeadersAndJson,
    pub remove: HeadersAndJson,
    pub replace: HeadersAndJson,
}

impl ResponseTransformerPluginConfig {
    fn new(headers: BTreeMap<String, String>) -> Self {
        let mut headers_strs = Vec::new();
        for (k, v) in headers {
            headers_strs.push(format!("{}: {}", k, v));
        }
        ResponseTransformerPluginConfig {
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

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TcpLogPluginConfig {
    pub host: String,
    pub port: u32,
    pub keepalive: u32,
    pub timeout: u32,
}

impl TcpLogPluginConfig {
    fn new(host: &str, port: u32) -> Self {
        TcpLogPluginConfig {
            host: host.into(),
            port: port.into(),
            keepalive: 60000,
            timeout: 10000,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Oauth2PluginConfig {
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

impl Oauth2PluginConfig {
    fn new(kong_token_expiration: u32, oauth_provision_key: &str, anonymous_consumer: Option<String>) -> Self {
        Oauth2PluginConfig {
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
            ..Oauth2PluginConfig::default()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Oauth2ExtensionPluginConfig {}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct JwtPluginConfig {
    pub uri_param_names: Vec<String>,
    pub claims_to_verify: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anonymous_username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anonymous: Option<String>,
}
impl JwtPluginConfig {
    fn new(anonymous_consumer: Option<String>) -> Self {
        JwtPluginConfig {
            uri_param_names: vec![],
            claims_to_verify: vec!["exp".into()],

            anonymous: match anonymous_consumer.clone() {
                Some(_s) => None,
                None     => Some("".into()),
            },
            anonymous_username: anonymous_consumer.map(|_| "anonymous".into()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct JsonCookiesCsrfPluginConfig {
    pub csrf_field_name: String,
    pub cookie_name: String,
    pub strict: bool,
    pub csrf_header_name: String,
}
impl Default for JsonCookiesCsrfPluginConfig {
    fn default() -> Self {
        JsonCookiesCsrfPluginConfig {
            cookie_name: "autologin_info".into(),
            csrf_field_name: "csrf_token".into(),
            csrf_header_name: "x-security-token".into(),
            strict: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct JsonCookiesToHeadersPluginConfig {
    pub field_name: String,
    pub cookie_name: String,
}

impl Default for JsonCookiesToHeadersPluginConfig {
    fn default() -> Self {
        JsonCookiesToHeadersPluginConfig {
            field_name: "kong_token".into(),
            cookie_name: "autologin_token".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct BabylonAuthHeaderPluginConfig {
    pub auth_service: String,
    pub cache_timeout_sec: u32,
    pub http_timeout_msec: u32,
}

impl BabylonAuthHeaderPluginConfig {
    fn new(authheader: BabylonAuthHeader) -> Self {
        BabylonAuthHeaderPluginConfig {
            auth_service: authheader.auth_service,
            cache_timeout_sec: authheader.cache_timeout_sec,
            http_timeout_msec: authheader.http_timeout_msec,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct CorrelationIdPluginConfig {
    pub echo_downstream: bool,
    pub header_name: String,
    pub generator: String,
}

impl Default for CorrelationIdPluginConfig {
    fn default() -> Self {
        CorrelationIdPluginConfig {
            echo_downstream: true,
            header_name: "babylon-request-id".into(),
            generator: "uuid".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "name", rename_all = "kebab-case")]
pub enum ApiPlugin {
    TcpLog(PluginBase<TcpLogPluginConfig>),
    Oauth2(PluginBase<Oauth2PluginConfig>),
    Oauth2Extension(PluginBase<Oauth2ExtensionPluginConfig>),
    Jwt(PluginBase<JwtPluginConfig>),
    Cors(PluginBase<CorsPluginConfig>),
    CorrelationId(PluginBase<CorrelationIdPluginConfig>),
    BabylonAuthHeader(PluginBase<BabylonAuthHeaderPluginConfig>),
    JsonCookiesToHeaders(PluginBase<JsonCookiesToHeadersPluginConfig>),
    JsonCookiesCsrf(PluginBase<JsonCookiesCsrfPluginConfig>),
    ResponseTransformer(PluginBase<ResponseTransformerPluginConfig>),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "ensure", content = "attributes", rename_all = "lowercase")]
pub enum PluginBase<T> {
    Present(PluginAttributes<T>),
    Removed,
}

impl<T: Default> Default for PluginBase<T> {
    fn default() -> Self { PluginBase::new(T::default()) }
}

impl<T> PluginBase<T> {
    fn new(config: T) -> Self {
        PluginBase::Present(PluginAttributes {
            enabled: true,
            config: config,
        })
    }
    fn removed() -> Self {
        PluginBase::Removed
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PluginAttributes<T> {
    pub enabled: bool,
    pub config: T,
}

impl<T: Default> Default for PluginAttributes<T> {
    fn default() -> Self {
        PluginAttributes {
            enabled: true,
            config: T::default()
        }
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
        plugins.push(ApiPlugin::CorrelationId(PluginBase::default()));

        // If globally enabled: TCP Logging
        if config.tcp_log.enabled {
            plugins.push(ApiPlugin::TcpLog(PluginBase::new(
                TcpLogPluginConfig::new(&config.tcp_log.host, config.tcp_log.port.parse().unwrap()),
            )));
        }

        // OAuth2 plugins
        let plugin = match v.auth {
            Authentication::OAuth2 => PluginBase::new(Oauth2PluginConfig::new(
                config.kong_token_expiration,
                &config.oauth_provision_key,
                v.oauth2_anonymous.clone())),
            _ => PluginBase::removed(),
        };
        plugins.push(ApiPlugin::Oauth2(plugin));

        // JWT plugin
        let plugin = match v.auth {
            Authentication::Jwt => PluginBase::new(JwtPluginConfig::new(
                v.oauth2_anonymous.clone(),
            )),
            _ => PluginBase::removed(),
        };
        plugins.push(ApiPlugin::Jwt(plugin));

        // OAuth2 extension plugin
        // TODO: Remove plugin if not Some(false)/None
        if let Some(true) = v.oauth2_extension_plugin {
            plugins.push(ApiPlugin::Oauth2Extension(PluginBase::default()));
        }

        // Babylon Auth Header plugin
        // TODO: Remove plugin if not enabled/None
        if let Some(babylon_auth_header) = v.babylon_auth_header {
            let plugin = PluginBase::Present(PluginAttributes {
                enabled: babylon_auth_header.enabled,
                config: BabylonAuthHeaderPluginConfig::new(babylon_auth_header),
            });
            plugins.push(ApiPlugin::BabylonAuthHeader(plugin));
        }

        // If enabled: CORS
        if let Some(cors) = v.cors {
            plugins.push(ApiPlugin::Cors(PluginBase::Present(PluginAttributes {
                // TODO: Remove plugin if not enabled/None
                enabled: cors.enabled,
                config: CorsPluginConfig::new(cors),
            })));
        }

        // If enabled: ResponseTransformer to add headers
        if let Some(add_headers) = v.add_headers {
            plugins.push(ApiPlugin::ResponseTransformer(PluginBase::new(
                ResponseTransformerPluginConfig::new(add_headers),
            )));
        }

        // If enabled: JsonCookies and JsonCookiesCsrf
        if v.cookie_auth {
            plugins.push(ApiPlugin::JsonCookiesToHeaders(PluginBase::default()));
        }

        if v.cookie_auth_csrf {
            plugins.push(ApiPlugin::JsonCookiesCsrf(PluginBase::default()));
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
