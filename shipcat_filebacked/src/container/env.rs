use merge::Merge;
use std::collections::BTreeMap;

use shipcat_definitions::Result;
use shipcat_definitions::structs::EnvVars;

use crate::util::{Build, RelaxedString};

#[derive(Deserialize, Clone, Default, Debug, PartialEq, Merge)]
pub struct EnvVarsSource(BTreeMap<String, RelaxedString>);

impl Build<EnvVars, ()> for EnvVarsSource {
    fn build(self, params: &()) -> Result<EnvVars> {
        let Self(plain) = self;
        let env = EnvVars::new(plain.build(params)?);
        // TODO: Inline
        env.verify()?;
        Ok(env)
    }
}

impl<K: ToString, V: Into<RelaxedString>> From<BTreeMap<K, V>> for EnvVarsSource {
    fn from(v: BTreeMap<K, V>) -> Self {
        let mut env = BTreeMap::new();
        for (k, v) in v {
            env.insert(k.to_string(), v.into());
        }
        EnvVarsSource(env)
    }
}
