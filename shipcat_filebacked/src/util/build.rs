use shipcat_definitions::Result;
use std::collections::BTreeMap;

pub trait Build<T, P> {
    fn build(self, params: &P) -> Result<T>;
}

impl<T, P, S: Build<T, P>> Build<Option<T>, P> for Option<S> {
    fn build(self, params: &P) -> Result<Option<T>> {
        self.map(|s| s.build(params)).transpose()
    }
}

impl<T, P, S: Build<T, P>> Build<Vec<T>, P> for Vec<S> {
    fn build(self, params: &P) -> Result<Vec<T>> {
        self.into_iter().map(|s| s.build(params)).collect()
    }
}

impl<V, P, S: Build<V, P>> Build<BTreeMap<String, V>, P> for BTreeMap<String, S> {
    fn build(self, params: &P) -> Result<BTreeMap<String, V>> {
        self.into_iter()
            .map(|(k, s)| s.build(params).map(|v| (k, v)))
            .collect()
    }
}
