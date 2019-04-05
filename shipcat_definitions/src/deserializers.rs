use std::fmt;
use std::marker::PhantomData;
use serde::de::{Visitor, Deserialize, Deserializer, Error, SeqAccess};
use serde::de::value::{SeqAccessDeserializer};

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
mod relaxed_string_tests {
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

#[derive(Deserialize, Clone, Default)]
pub struct CommaSeparatedString(
    #[serde(deserialize_with="comma_separated_string")]
    Vec<String>
);

impl Into<Vec<String>> for CommaSeparatedString {
    fn into(self) -> Vec<String> {
        self.0
    }
}

pub fn comma_separated_string<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>
{

    struct CommaSeparatedString(PhantomData<fn() -> Vec<String>>);

    impl<'de> Visitor<'de> for CommaSeparatedString {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("comma-separated string or list")
        }

        fn visit_str<E>(self, value: &str) -> Result<Vec<String>, E>
        where
            E: Error,
        {
            Ok(value.split(",").map(String::from).collect())
        }

        fn visit_seq<A>(self, seq: A) -> Result<Vec<String>, A::Error>
        where
            A: SeqAccess<'de>,
        {
            Deserialize::deserialize(SeqAccessDeserializer::new(seq))
        }
    }
    deserializer.deserialize_any(CommaSeparatedString(PhantomData))
}

#[cfg(test)]
mod comma_separated_string_tests {
    use super::{CommaSeparatedString};

    #[test]
    fn deserialize_single_string() {
        let CommaSeparatedString(x) = serde_yaml::from_str("'foo'").unwrap();
        assert_eq!(x, vec!["foo".to_string()]);
    }

    #[test]
    fn deserialize_comma_separated_string() {
        let CommaSeparatedString(x) = serde_yaml::from_str("'foo,bar,blort'").unwrap();
        assert_eq!(x, vec!["foo".to_string(), "bar".to_string(), "blort".to_string()]);
    }

    #[test]
    fn deserialize_empty_list() {
        let CommaSeparatedString(x) = serde_yaml::from_str("[]").unwrap();
        assert_eq!(x, Vec::<String>::new());
    }

    #[test]
    fn deserialize_list() {
        let CommaSeparatedString(x) = serde_yaml::from_str("[foo,bar,blort]").unwrap();
        assert_eq!(x, vec!["foo".to_string(), "bar".to_string(), "blort".to_string()]);
    }
}
