use structs::kong::Kong;
use std::collections::BTreeMap;

use semver::Version;

use super::Vault;
#[allow(unused_imports)]
use super::{Result, Error};

/// Versioning Scheme used in region
///
/// This is valdiated strictly using `shipcat validate` when versions are found in manifests.
/// Otherwise, it's validated on upgrade time (via `shipcat apply`) when it's passed.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum VersionScheme {
    /// Version must be valid semver (no leading v)
    ///
    /// This is the assumed default for regions that lock versions in manifests.
    Semver,
    /// Version must be valid semver or a 40 character hex (git sha)
    ///
    /// This can be used for rolling environments that does not lock versions in manifests.
    GitShaOrSemver,
}

impl Default for VersionScheme {
    fn default() -> VersionScheme {
        VersionScheme::Semver
    }
}

/// Version validator
impl VersionScheme {
    pub fn verify(&self, ver: &str) -> Result<()> {
        use regex::Regex;
        let gitre = Regex::new(r"^[0-9a-f\-]{40}$").unwrap();
        match *self {
            VersionScheme::GitShaOrSemver => {
                if !gitre.is_match(&ver) && Version::parse(&ver).is_err() {
                    bail!("Illegal tag {} (floating tags cannot be rolled back please use 40 char git sha or semver)", ver);
                }
            },
            VersionScheme::Semver => {
                if Version::parse(&ver).is_err() {
                    bail!("Version {} is not a semver version in a region using semver versions", ver);
                }
            },
        };
        Ok(())
    }
}

/// Vault configuration for a region
#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(Default))]
#[serde(deny_unknown_fields)]
pub struct VaultConfig {
    /// Vault url up to and including port
    pub url: String,
    /// Root folder under secret/
    ///
    /// Typically, the name of the region to disambiguate.
    pub folder: String,
}

//#[derive(Serialize, Deserialize, Clone, Default)]
//#[serde(deny_unknown_fields)]
//pub struct HostPort {
//    /// Hostname || IP || FQDN
//    pub host: String,
//    /// Port
//    pub port: u32,
//}

/// Kafka configuration for a region
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct KafkaConfig {
    /// Broker urls in "hostname:port" format.
    ///
    /// These are injected in to the manifest.kafka struct if it's set.
    pub brokers: Vec<String>,
}

// ----------------------------------------------------------------------------------

/// Kong configuration for a region
#[derive(Serialize, Deserialize, Clone, Default)] // TODO: better Default impl
#[serde(deny_unknown_fields)]
pub struct KongConfig {
    /// Base URL to use (e.g. uk.dev.babylontech.co.uk)
    pub base_url: String,
    /// Configuration API URL (e.g. https://kong-admin-ops.dev.babylontech.co.uk)
    pub config_url: String,
    /// Kong token expiration time (in seconds)
    pub kong_token_expiration: u32,
    pub oauth_provision_key: String,
    /// TCP logging options
    pub tcp_log: KongTcpLogConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anonymous_consumers: Option<KongAnonymousConsumers>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub consumers: BTreeMap<String, KongOauthConsumer>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub internal_ips_whitelist: Vec<String>,
    #[serde(default, skip_serializing)]
    pub extra_apis: BTreeMap<String, Kong>,
}

/// Logz.io configuration for a region
#[derive(Serialize, Deserialize, Clone, Default)] // TODO: better Default impl
#[serde(deny_unknown_fields)]
pub struct LogzIoConfig {
    /// Base URL to use (e.g. https://app-eu.logz.io/#/dashboard/kibana/dashboard)
    pub url: String,
    /// Account ID (e.g. 46609)
    pub account_id: String,
}

/// Grafana details for a region
#[derive(Serialize, Deserialize, Clone, Default)] // TODO: better Default impl
#[serde(deny_unknown_fields)]
pub struct GrafanaConfig {
    /// Base URL to use (e.g. https://dev-grafana.ops.babylontech.co.uk)
    pub url: String,
    /// Services Dashboard ID (e.g. oHzT4g0iz)
    pub services_dashboard_id: String,
}

/// Sentry details for a region
#[derive(Serialize, Deserialize, Clone, Default)] // TODO: better Default impl
#[serde(deny_unknown_fields)]
pub struct SentryConfig {
    /// Base URL to use (e.g. https://dev-uk-sentry.ops.babylontech.co.uk)
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct KongAnonymousConsumers {
    pub anonymous: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct KongOauthConsumer {
    pub oauth_client_id: String,
    pub oauth_client_secret: String,
    pub username: String
}

impl KongConfig {
    fn secrets(&mut self, vault: &Vault, region: &str) -> Result<()> {
        for (svc, data) in &mut self.consumers {
            if data.oauth_client_id == "IN_VAULT" {
                let vkey = format!("{}/kong/consumers/{}_oauth_client_id", region, svc);
                data.oauth_client_id = vault.read(&vkey)?;
            }
            if data.oauth_client_secret == "IN_VAULT" {
                let vkey = format!("{}/kong/consumers/{}_oauth_client_secret", region, svc);
                data.oauth_client_secret = vault.read(&vkey)?;
            }
        }
        if self.oauth_provision_key == "IN_VAULT" {
            let vkey = format!("{}/kong/oauth_provision_key", region);
            self.oauth_provision_key = vault.read(&vkey)?;
        }
        Ok(())
    }
    fn verify_secrets_exist(&self, vault: &Vault, region: &str) -> Result<()> {
        let mut expected = vec![];
        for (svc, data) in &self.consumers {
            if data.oauth_client_id == "IN_VAULT" {
                expected.push(format!("{}_oauth_client_id", svc));
            }
            if data.oauth_client_secret == "IN_VAULT" {
                expected.push(format!("{}_oauth_client_secret", svc));
            }
        }
        if expected.is_empty() {
            return Ok(()); // no point trying to cross reference
        }
        let secpth = format!("{}/kong/consumers", region);
        let found = vault.list(&secpth)?;
        debug!("Found kong secrets {:?} for {}", found, region);
        for v in expected {
            if !found.contains(&v) {
                bail!("Kong secret {} not found in {} vault", v, region);
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct KongTcpLogConfig {
    pub enabled: bool,
    pub host: String,
    pub port: String,
}

impl KongConfig {
    pub fn verify(&self) -> Result<()> {
        Ok(())
    }
}

// ----------------------------------------------------------------------------------

/// A region is an abstract kube context
///
/// Either it's a pure kubernetes context with a namespace and a cluster,
/// or it's an abstract concept with many associated real kubernetes contexts.
#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(Default))]
#[serde(deny_unknown_fields)]
pub struct Region {
    /// Name of region
    pub name: String,
    /// Kubernetes namespace
    pub namespace: String,
    /// Environment (e.g. `dev` or `staging`)
    pub environment: String,
    /// Versioning scheme
    pub versioningScheme: VersionScheme,

    /// Important base urls that can be templated in evars
    #[serde(default)]
    pub base_urls: BTreeMap<String, String>,

    /// Environment variables to inject
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Kong configuration for the region
    #[serde(default)]
    pub kong: KongConfig,
    /// List of Whitelisted IPs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ip_whitelist: Vec<String>,
    /// Kafka configuration for the region
    #[serde(default)]
    pub kafka: KafkaConfig,
    /// Vault configuration for the region
    pub vault: VaultConfig,
    /// Logz.io configuration for the region
    pub logzio: Option<LogzIoConfig>,
    /// Grafana details for the region
    pub grafana: Option<GrafanaConfig>,
    /// Sentry URL for the region
    pub sentry: Option<SentryConfig>,
    /// List of locations the region serves
    #[serde(default)]
    pub locations: Vec<String>,
}

impl Region {
    // Internal secret populator for Config::new
    pub fn secrets(&mut self) -> Result<()> {
        let v = Vault::regional(&self.vault)?;
        self.kong.secrets(&v, &self.name)?;
        Ok(())
    }

    // Entry point for region verifier
    pub fn verify_secrets_exist(&mut self) -> Result<()> {
        let v = Vault::regional(&self.vault)?;
        debug!("Validating kong secrets for {}", self.name);
        self.kong.verify_secrets_exist(&v, &self.name)?;
        Ok(())
    }

    // Get the Vault URL for a given service in this region
    pub fn vault_url(&self, app: &str) -> String {
        // We use different UIs whether its the "classic vault" or the "regional vault"
        let mut vault_url = self.vault.url.clone();
        let path = if vault_url.contains("8200") {
            vault_url = vault_url.replace("8200", "");
            "/secrets/generic/secret/"
        } else {
            "/ui/vault/secrets/secret/list/"
        };

        format!("{vault_url}/{path}/{env}/{app}/",
            vault_url = vault_url.trim_matches('/'),
            path = path.trim_matches('/'),
            env = &self.name,
            app = &app)
    }

    pub fn grafana_url(&self, app: &str, cluster: &str) -> Option<String> {
        self.grafana.clone().map(|gf| {
            format!("{grafana_url}/d/{dashboard_id}/kubernetes-services?var-cluster={cluster}&var-namespace={namespace}&var-deployment={app}",
              grafana_url = gf.url.trim_matches('/'),
              dashboard_id = gf.services_dashboard_id,
              app = app,
              cluster = cluster,
              namespace = &self.namespace)
        })
    }

    // Get the Sentry URL for a given service slug in a cluster in this region
    pub fn sentry_url(&self, slug: &str) -> Option<String> {
        self.sentry.clone().map(|s| {
            format!("{sentry_base_url}/sentry/{slug}",
                    sentry_base_url = s.url, slug = slug)
        })
    }

    pub fn logzio_url(&self, app: &str) -> Option<String> {
        self.logzio.clone().map(|lio| {
            format!("{logzio_url}/{app}-{env}?accountIds={account_id}",
              logzio_url = lio.url.trim_matches('/'),
              app = app,
              env = self.name,
              account_id = lio.account_id)
        })
    }
}
