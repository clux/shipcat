use regex::Regex;

use shipcat_definitions::Result;

use crate::util::Build;

#[derive(Deserialize, Clone)]
pub struct ImageNameSource(String);

impl Build<String, ()> for ImageNameSource {
    fn build(self, _: &()) -> Result<String> {
        let Self(image) = self;
        // https://docs.docker.com/engine/reference/commandline/tag/
        // An image name is made up of slash-separated name components, optionally prefixed by a registry hostname.
        // The hostname must comply with standard DNS rules, but may not contain underscores.
        // If a hostname is present, it may optionally be followed by a port number in the format :8080.
        // If not present, the command uses Docker’s public registry located at registry-1.docker.io by default.
        // Name components may contain lowercase letters, digits and separators.
        // A separator is defined as a period, one or two underscores, or one or more dashes.
        // A name component may not start or end with a separator.
        let re = Regex::new(r"^([^:/_]+(:\d+)?/)?([a-z\d._-]+/)*[a-z\d._-]+$").unwrap();
        if !re.is_match(&image) {
            bail!("The image {} does not match a valid image repository", image);
        }
        Ok(image)
    }
}

#[derive(Deserialize, Clone)]
pub struct ImageTagSource(String);

impl Build<String, ()> for ImageTagSource {
    fn build(self, _: &()) -> Result<String> {
        let Self(tag) = self;
        // https://docs.docker.com/engine/reference/commandline/tag/
        // A tag name must be valid ASCII and may contain lowercase and uppercase letters, digits, underscores, periods and dashes.
        // A tag name may not start with a period or a dash and may contain a maximum of 128 characters.
        let re = Regex::new(r"^[[:alpha:]\d][[:alpha:]\d\-_.]{0,127}$").unwrap();
        if !re.is_match(&tag) {
            bail!("The tag {} is not a valid Docker image tag", tag);
        }
        Ok(tag)
    }
}

#[cfg(test)]
mod tests {
    use super::{ImageNameSource, ImageTagSource};
    use crate::util::Build;

    macro_rules! assert_valid {
        ( $source_type:path, $expected:expr ) => {{
            let source = $source_type($expected.to_string());
            let actual = source.build(&()).unwrap();
            assert_eq!($expected, actual);
        }};
    }

    #[test]
    fn tags() {
        assert_valid!(ImageTagSource, "latest");
        assert_valid!(ImageTagSource, "master");
        assert_valid!(ImageTagSource, "a".repeat(128));
        assert_valid!(ImageTagSource, "0123");
        assert_valid!(ImageTagSource, "1.2.3-beta_456");

        ImageTagSource("foo/bar".to_string()).build(&()).unwrap_err();
        ImageTagSource("bar:latest".to_string()).build(&()).unwrap_err();
    }

    #[test]
    fn names() {
        assert_valid!(ImageNameSource, "alpine");
        assert_valid!(ImageNameSource, "circleci/ruby");

        assert_valid!(ImageNameSource, "quay.io/circleci/ruby");
        assert_valid!(ImageNameSource, "quay.io:80/foo");
        assert_valid!(ImageNameSource, "quay.io:80/foo/bar");
        assert_valid!(ImageNameSource, "quay.io:80/foo/bar/baz");

        ImageNameSource("alpine:latest".to_string())
            .build(&())
            .unwrap_err();
        ImageNameSource("foo/bar:latest".to_string())
            .build(&())
            .unwrap_err();
    }
}
