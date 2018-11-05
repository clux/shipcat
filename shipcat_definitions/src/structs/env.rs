use std::collections::{btree_map, BTreeMap};
use std::ops::{Deref, DerefMut};
use std::iter::IntoIterator;
use super::Result;

// An attempt at generalising Env vars - whose BTreeMap logic was splattered over
// the Manifest module. Now we define a newtype. And implement methods on that:

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
