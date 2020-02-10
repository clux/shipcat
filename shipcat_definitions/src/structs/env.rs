use super::Result;
use std::{
    collections::{BTreeMap, BTreeSet},
    mem,
};

/// Environment variables to inject
///
/// These have a few special convenience behaviours:
/// "IN_VAULT" values is replaced with value from vault/secret/folder/service/KEY
/// One off `tera` templates are calculated with a limited template context
///
/// IN_VAULT secrets will all be put in a single kubernetes `Secret` object.
/// One off templates **can** be put in a `Secret` object if marked `| as_secret`.
///
/// ```yaml
/// env:
///   # plain eva:
///   PLAIN_EVAR: plaintextvalue
///
///   # vault lookup:
///   DATABASE_URL: IN_VAULT
///
///   # templated evars:
///   INTERNAL_AUTH_URL: "{{ base_urls.services }}/auth/internal"
/// ```
///
/// The vault lookup will GET from the region specific path for vault, in the
/// webapp subfolder, getting the `DATABASE_URL` secret.
///
/// The `kong` templating will use the secrets read from the `Config` for this
/// region, and replace them internally.
///
/// The `as_secret` destinction only serves to put `AUTH_SECRET` into `Manifest::secrets`.
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(default)]
pub struct EnvVars {
    /// Plain text (non-secret) environment variables
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub plain: BTreeMap<String, String>,

    /// Environment variable names stored in secrets
    ///
    /// This is an internal property that is exposed as an output only.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub secrets: BTreeSet<String>,
}

impl EnvVars {
    pub fn new(env: BTreeMap<String, String>) -> Self {
        EnvVars {
            plain: env,
            secrets: Default::default(),
        }
    }

    fn is_vault_secret(value: &str) -> bool {
        value == "IN_VAULT"
    }

    fn template_secret_value(value: &str) -> Option<String> {
        let prefix = "SHIPCAT_SECRET::";
        if value.starts_with(prefix) {
            Some(value.to_string().split_off(prefix.len()))
        } else {
            None
        }
    }

    pub fn verify(&self) -> Result<()> {
        for k in self.plain.keys() {
            if k != &k.to_uppercase() {
                bail!("Env vars need to be uppercase, found: {}", k);
            }
        }
        Ok(())
    }

    // Remove variables with a value "IN_VAULT", mark them as a secret and return them.
    pub fn vault_secrets(&mut self) -> BTreeSet<String> {
        let mut plain = BTreeMap::new();
        let mut vs = BTreeSet::new();
        for (k, v) in self.plain.iter() {
            if EnvVars::is_vault_secret(&v) {
                vs.insert(k.to_string());
                self.secrets.insert(k.to_string());
            } else {
                plain.insert(k.to_string(), v.to_string());
            }
        }
        mem::replace(&mut self.plain, plain);
        vs
    }

    // Remove secrets generated from templates from the plain variables, mark them as a secret and return them.
    pub fn template_secrets(&mut self) -> BTreeMap<String, String> {
        let mut plain = BTreeMap::new();
        let mut ts = BTreeMap::new();
        for (k, v) in self.plain.iter() {
            match EnvVars::template_secret_value(v) {
                Some(x) => {
                    ts.insert(k.to_string(), x);
                    self.secrets.insert(k.to_string());
                }
                None => {
                    plain.insert(k.to_string(), v.to_string());
                }
            };
        }
        mem::replace(&mut self.plain, plain);
        ts
    }
}
