use merge::Merge;
use shipcat_definitions::Result;

use super::Build;

/// Enabled wraps a struct and adds an `enabled` field.
#[derive(Deserialize, Default, Clone, PartialEq, Merge)]
#[cfg_attr(test, derive(Debug, Copy))]
#[serde(default)]
pub struct Enabled<T: Merge> {
    pub enabled: Option<bool>,

    #[serde(flatten)]
    pub item: T,
}

/// Builds the inner struct unless enabled is explicitly false.
impl<S: Build<B, P> + Merge, B, P> Build<Option<B>, P> for Enabled<S> {
    fn build(self, params: &P) -> Result<Option<B>> {
        match self.enabled {
            Some(false) => Ok(None),
            _ => Ok(Some(self.item.build(params)?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use merge::Merge;
    use shipcat_definitions::Result;

    use super::{Build, Enabled};

    #[derive(Deserialize)]
    struct Item(&'static str);

    impl Build<String, u32> for Item {
        fn build(self, params: &u32) -> Result<String> {
            Ok(format!("{}/{}", self.0, params))
        }
    }

    impl Merge for Item {
        fn merge(self, _: Self) -> Self {
            panic!("Not implemented: unused by tests");
        }
    }

    #[test]
    fn build() {
        let source = Enabled {
            enabled: None,
            item: Item("foo"),
        };
        assert_eq!(source.build(&1).unwrap(), Some("foo/1".into()));

        let source = Enabled {
            enabled: Some(true),
            item: Item("bar"),
        };
        assert_eq!(source.build(&2).unwrap(), Some("bar/2".into()));

        let source = Enabled {
            enabled: Some(false),
            item: Item("blort"),
        };
        assert_eq!(source.build(&3).unwrap(), None);
    }

    #[test]
    fn merge() {
        let empty = Enabled::<Option<u32>> {
            enabled: None,
            item: None,
        };
        let full = Enabled {
            enabled: Some(true),
            item: Some(2),
        };
        let partial1 = Enabled {
            enabled: Some(false),
            item: None,
        };
        let partial2 = Enabled {
            enabled: None,
            item: Some(4),
        };

        // x.merge(x) == x
        assert_eq!(empty.merge(empty), empty);
        assert_eq!(full.merge(full), full);
        assert_eq!(partial1.merge(partial1), partial1);
        assert_eq!(partial2.merge(partial2), partial2);

        // empty.merge(x) == x
        assert_eq!(empty.merge(full), full);
        assert_eq!(empty.merge(partial1), partial1);
        assert_eq!(empty.merge(partial2), partial2);

        // x.merge(empty) == x
        assert_eq!(full.merge(empty), full);
        assert_eq!(partial1.merge(empty), partial1);
        assert_eq!(partial2.merge(empty), partial2);

        // x.merge(full) == full
        assert_eq!(partial1.merge(full), full);
        assert_eq!(partial2.merge(full), full);

        assert_eq!(full.merge(partial1), Enabled {
            enabled: Some(false),
            item: Some(2),
        });
        assert_eq!(full.merge(partial2), Enabled {
            enabled: Some(true),
            item: Some(4),
        });
        assert_eq!(partial1.merge(partial2), Enabled {
            enabled: Some(false),
            item: Some(4),
        });
        assert_eq!(partial2.merge(partial1), Enabled {
            enabled: Some(false),
            item: Some(4),
        });
    }
}
