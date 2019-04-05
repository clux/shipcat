use merge::Merge;
use std::collections::BTreeMap;

use shipcat_definitions::structs::{Authentication, Authorization, BabylonAuthHeader, Cors, Kong};
use shipcat_definitions::{Region, Result};
use shipcat_definitions::deserializers::{CommaSeparatedString};

use super::authorization::AuthorizationSource;

/// Main manifest, deserialized from `shipcat.yml`.
#[derive(Deserialize, Default, Merge, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct KongSource {
    pub upstream_url: Option<String>,
    pub uris: Option<String>,
    // TODO: Breaking change to Option<Vec<String>>
    pub hosts: Option<CommaSeparatedString>,
    pub host: Option<String>,
    pub strip_uri: Option<bool>,
    pub preserve_host: Option<bool>,
    pub cors: Option<Cors>,
    pub additional_internal_ips: Option<Vec<String>>,

    pub internal: Option<bool>,
    #[serde(rename = "camelCase")]
    pub publicly_accessible: Option<bool>,
    pub unauthenticated: Option<bool>,
    pub cookie_auth: Option<bool>,
    pub cookie_auth_csrf: Option<bool>,
    pub auth: Option<Authentication>,
    pub babylon_auth_header: Option<BabylonAuthHeader>,
    pub oauth2_anonymous: Option<String>,
    pub oauth2_extension_plugin: Option<bool>,
    pub authorization: AuthorizationSource,

    pub upstream_connect_timeout: Option<u32>,
    pub upstream_send_timeout: Option<u32>,
    pub upstream_read_timeout: Option<u32>,
    pub add_headers: BTreeMap<String, String>,

    pub name: Option<String>,
}

impl KongSource {
    /// Build a Kong from a KongSource, validating and mutating properties.
    pub fn build(self, service: &String, region: &Region, hosts: Option<Vec<String>>) -> Result<Option<Kong>> {
        let hosts = self.build_hosts(region, hosts.unwrap_or_default())?;

        if hosts.is_empty() && self.uris.is_none() {
            return Ok(None);
        }

        let upstream_url = self.build_upstream_url(service, region);
        let (auth, authorization) = self.build_auth()?;

        if self.name.is_some() {
            warn!("kong.name is ignored");
        }

        if authorization.is_some() {
            if self.cookie_auth.is_some() {
                bail!("cookie_auth and authorization properties are mutually exclusive")
            }
            if self.cookie_auth_csrf.is_some() {
                bail!("cookie_auth_csrf and authorization properties are mutually exclusive")
            }
            if self.oauth2_anonymous.is_some() {
                bail!("oauth2_anonymous and authorization properties are mutually exclusive")
            }
            if self.oauth2_extension_plugin.is_some() {
                bail!("oauth2_extension_plugin and authorization properties are mutually exclusive")
            }
        }

        Ok(Some(Kong {
            name: service.to_string(),
            upstream_url: upstream_url,
            internal: self.internal.unwrap_or_default(),
            publiclyAccessible: self.publicly_accessible.unwrap_or_default(),
            uris: self.uris,
            hosts,
            authorization,
            strip_uri: self.strip_uri.unwrap_or_default(),
            preserve_host: self.preserve_host.unwrap_or(true),
            cors: self.cors,
            additional_internal_ips: self.additional_internal_ips.unwrap_or_default(),
            babylon_auth_header: self.babylon_auth_header,
            upstream_connect_timeout: self.upstream_connect_timeout,
            upstream_send_timeout: self.upstream_send_timeout,
            upstream_read_timeout: self.upstream_read_timeout,
            add_headers: self.add_headers,
            // Legacy authorization
            auth,
            cookie_auth: self.cookie_auth.unwrap_or_default(),
            cookie_auth_csrf: self.cookie_auth_csrf.unwrap_or_default(),
            oauth2_anonymous: self.oauth2_anonymous,
            oauth2_extension_plugin: self.oauth2_extension_plugin,
        }))
    }

    fn build_upstream_url(&self, service: &String, region: &Region) -> String {
        if let Some(upstream_url) = &self.upstream_url {
            upstream_url.to_string()
        } else {
            format!("http://{}.{}.svc.cluster.local", service, region.namespace)
        }
    }

    fn build_auth(&self) -> Result<(Authentication, Option<Authorization>)> {
        let authorization = self.authorization.clone().build()?;
        match (
            &self.auth,
            self.unauthenticated.unwrap_or(false),
            authorization,
        ) {
            // unauthenticated is true
            (None, true, None) => Ok((Authentication::None, None)),
            (Some(_), true, _) => {
                bail!("unauthenticated and auth properties are mutually exclusive")
            }
            (_, true, Some(_)) => {
                bail!("unauthenticated and authorization properties are mutually exclusive")
            }
            // authorization is enabled
            (None, _, Some(a)) | (Some(Authentication::Jwt), _, Some(a)) => {
                Ok((Authentication::Jwt, Some(a)))
            }
            (Some(_), _, Some(_)) => bail!("auth must be unset or JWT if authorization is enabled"),
            // otherwise
            (Some(x), _, _) => Ok((x.clone(), None)),
            (None, _, _) => Ok((Authentication::default(), None)),
        }
    }

    fn build_hosts(&self, region: &Region, tophosts: Vec<String>) -> Result<Vec<String>> {
        let hosts: Vec<String> = self.hosts.clone().unwrap_or_default().into();
        match (tophosts.as_slice(), &self.host, hosts.as_slice()) {
            (_, None, []) => Ok(tophosts),
            ([], None, _) => Ok(hosts),
            ([], Some(host), []) => Ok(vec![format!("{}{}", host, region.kong.base_url)]),
            (_, _, _) => bail!("hosts, kong.hosts and kong.host are mutually exclusive"),
        }
    }
}
