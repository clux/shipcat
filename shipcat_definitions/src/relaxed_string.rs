use std::fmt;

use serde::de::{Visitor, Deserialize, Deserializer, Error};

/// Strings, numbers and booleans can be deserialized into a RelaxedString
///
/// Serde will usually coerce these types into a string, but that doesn't work when combined with `flatten`
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RelaxedString(String);

impl ToString for RelaxedString {
    fn to_string(&self) -> String {
        let RelaxedString(x) = self;
        x.to_string()
    }
}

impl From<&String> for RelaxedString {
    fn from(v: &String) -> Self {
        Self(v.to_string())
    }
}

impl From<&str> for RelaxedString {
    fn from(v: &str) -> Self {
        Self(v.to_string())
    }
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error> where D: Deserializer<'de> {
    let RelaxedString(x) = RelaxedString::deserialize(deserializer)?;
    Ok(x)
}

impl<'de> Deserialize<'de> for RelaxedString {
    fn deserialize<D>(deserializer: D) -> Result<RelaxedString, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(RelaxedStringVisitor)
    }
}

struct RelaxedStringVisitor;

macro_rules! visit_tostring {
    ( $name:ident, $type:ty ) => {
        fn $name<E>(self, v: $type) -> Result<Self::Value, E> where E: Error {
            self.visit_string(v.to_string())
        }
    };
}

/// RelaxedStringVisitor will visit numbers, bools and string
impl<'de> Visitor<'de> for RelaxedStringVisitor {
    type Value = RelaxedString;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string, number, boolean or null")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E> where E: Error {
        Ok(RelaxedString(v))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> where E: Error {
        // This is weird behaviour, but matches existing Shipcat functionality
        Ok(RelaxedString("~".to_string()))
    }

    // Calls `self.visit_string(v.to_string())`
    visit_tostring!(visit_bool, bool);
    visit_tostring!(visit_str, &str);
    visit_tostring!(visit_i64, i64);
    visit_tostring!(visit_i128, i128);
    visit_tostring!(visit_u64, u64);
    visit_tostring!(visit_u128, u128);
    visit_tostring!(visit_f64, f64);
}

#[cfg(test)]
mod tests {
    use super::{RelaxedString};

    #[test]
    fn deserialize_string() {
        let RelaxedString(x) = serde_yaml::from_str("'foo'").unwrap();
        assert_eq!(x, "foo".to_string());
    }

    #[test]
    fn deserialize_integer() {
        let RelaxedString(x) = serde_yaml::from_str("123").unwrap();
        assert_eq!(x, "123".to_string());
    }

    #[test]
    fn deserialize_float() {
        let RelaxedString(x) = serde_yaml::from_str("1.3").unwrap();
        assert_eq!(x, "1.3".to_string());
    }

    #[test]
    fn deserialize_bool() {
        let RelaxedString(x) = serde_yaml::from_str("true").unwrap();
        assert_eq!(x, "true".to_string());
    }

    #[test]
    fn deserialize_null() {
        let RelaxedString(x) = serde_yaml::from_str("~").unwrap();
        assert_eq!(x, "~".to_string());
    }
}
