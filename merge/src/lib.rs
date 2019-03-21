use std::collections::BTreeMap;

pub trait Merge {
    /// Merge another instance into this one.
    ///
    /// Values defined in `other` take precedence over values defined in `self`.
    fn merge(self, other: Self) -> Self;
}

impl<T> Merge for Option<T> {
    #[inline]
    fn merge(self, other: Self) -> Self {
        other.or(self)
    }
}

// TODO: Merge values if defined in both?
impl<K: std::hash::Hash + Ord, V> Merge for BTreeMap<K, V> {
    fn merge(self, other: Self) -> Self {
        let mut merged = BTreeMap::new();
        for (k, v) in self.into_iter() {
            merged.insert(k, v);
        }
        for (k, v) in other.into_iter() {
            merged.insert(k, v);
        }
        merged
    }
}

#[cfg(test)]
mod tests {
    use crate::Merge;
    use std::collections::BTreeMap;

    #[test]
    fn option() {
        let a = Option::Some(1);
        let b = Option::Some(2);
        let none = Option::None;

        assert_eq!(a.merge(b), b);
        assert_eq!(a.merge(none), a);
        assert_eq!(none.merge(b), b);
        assert_eq!(none.merge(none), none);
    }

    #[test]
    fn btree_map() {
        let mut a = BTreeMap::new();
        a.insert("a", "a-value");
        a.insert("b", "a-value");

        let mut b = BTreeMap::new();
        b.insert("a", "b-value");
        b.insert("c", "b-value");

        let merged = a.merge(b);
        let mut expected = BTreeMap::new();
        expected.insert("a", "b-value");
        expected.insert("b", "a-value");
        expected.insert("c", "b-value");
        assert_eq!(merged, expected);
    }
}
