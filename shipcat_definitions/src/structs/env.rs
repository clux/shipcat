use std::collections::{btree_map, BTreeMap};
use std::ops::{Deref, DerefMut};
use std::iter::IntoIterator;
use std::iter::FromIterator;
use super::Result;

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
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct EnvVars(BTreeMap<String, String>);


impl EnvVars {
    pub fn verify(&self) -> Result<()> {
        for (k, v) in *&self {
            if v == "IN_VAULT" {
                // TODO: can do this generally here now
                bail!("Secret evars must go in the root service");
            }
            if k != &k.to_uppercase()  {
                bail!("Env vars need to be uppercase, found: {}", k);
            }
        }
        Ok(())
    }
}

// Convenience for serde skip_serializing_if
impl EnvVars {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// implementations of some std traits on this newtype below
// this it allows us to ues for loops on an EnvVars direct
// without having to access the `.0` element everywhere.

impl EnvVars {
    // this seems to work - although have to deref + ref (see verify)
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (&String, &String)> + 'a {
        self.0.iter()
    }

    //pub fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a str, &'a str)> + 'a {
    //    self.0.iter()
    //}
}


impl IntoIterator for EnvVars {
    type Item = <BTreeMap<String, String> as IntoIterator>::Item;
    type IntoIter = <BTreeMap<String, String> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a EnvVars {
    type Item = (&'a String, &'a String);
    type IntoIter = btree_map::Iter<'a, String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut EnvVars {
    type Item = (&'a String, &'a mut String);
    type IntoIter = btree_map::IterMut<'a, String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl FromIterator<(String, String)> for EnvVars {
    fn from_iter<I: IntoIterator<Item=(String, String)>>(iter: I) -> Self {
        let map = BTreeMap::from_iter(iter);

        EnvVars(map)
    }
}

impl Deref for EnvVars {
    type Target = BTreeMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EnvVars {
    fn deref_mut(&mut self) -> &mut BTreeMap<String, String> {
        &mut self.0
    }
}
