use std::collections::BTreeMap;

use super::{Resources};

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub struct Sidecar {
  pub name: String,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub version: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub resources: Option<Resources<String>>,

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
  ///   AUTH_ID: "{{ kong.consumers['webapp'].oauth_client_id }}"
  ///   AUTH_SECRET: "{{ kong.consumers['webapp'].oauth_client_secret | as_secret }}"
  /// ```
  ///
  /// The vault lookup will GET from the region specific path for vault, in the
  /// webapp subfolder, getting the `DATABASE_URL` secret.
  ///
  /// The `kong` templating will use the secrets read from the `Config` for this
  /// region, and replace them internally.
  ///
  /// The `as_secret` destinction only serves to put `AUTH_SECRET` into `Manifest::secrets`.
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  pub env: BTreeMap<String, String>,
}
