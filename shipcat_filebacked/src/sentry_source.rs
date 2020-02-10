use shipcat_definitions::{
    structs::{metadata::SlackChannel, sentry::Sentry},
    Result,
};

use crate::util::Build;

/// Monitoring section covering Sentry configuration
///
/// ```yaml
/// sentry:
///   # optional, defaults to team's notification channel
///   slack: C12ABYZ78
/// if you find sentry too noisy you are able to mute it with true
///   silent: true
/// ```
#[derive(Debug, Default, Clone, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SentrySource {
    /// if you find sentry too noisy you are able to mute it with true
    #[serde(default)]
    pub silent: bool,
    /// we might want to route only sentry to some dedicated channel
    pub slack: Option<SlackChannel>,
}

impl Build<Sentry, SlackChannel> for SentrySource {
    fn build(self, default_channel: &SlackChannel) -> Result<Sentry> {
        let slack = self
            .slack
            .map(|s| s.verify().map(|_| s))
            .unwrap_or_else(|| Ok(default_channel.clone()))?;

        if !slack.starts_with('C') {
            bail!(
                "Private/personal channel {} is NOT supported as Sentry target",
                *slack
            );
        }

        let silent = self.silent;

        Ok(Sentry { slack, silent })
    }
}

#[cfg(test)]
mod tests {
    use merge::Merge;
    use shipcat_definitions::{
        structs::{metadata::SlackChannel, sentry::Sentry},
        Result,
    };

    use super::{super::util::Build, SentrySource};

    //  make sure the macros are called as they are for actual/original structs
    #[derive(Deserialize, Default, Merge, Clone, Debug)]
    #[serde(default, deny_unknown_fields, rename_all = "camelCase")]
    pub struct ManifestOverridesNarrowed {
        #[serde(default)]
        sentry: Option<SentrySource>,
    }

    #[derive(Serialize, Deserialize, Clone, Default)]
    pub struct ManifestNarrowed {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub sentry: Option<Sentry>,
    }

    pub struct TestSet {
        manifest_yml: String,
        env_yml: String,
        default_channel: String,
        crd_expected: String,
    }

    #[test]
    fn test_empty_all_the_things() -> Result<()> {
        test_parse_and_merge(TestSet {
            manifest_yml: "{}".into(),
            env_yml: "{}".into(),
            default_channel: "CDEFTEAM8".into(),
            crd_expected: r###"---
{}"###
                .into(),
        })
    }

    #[test]
    fn test_default_slack_propagated() -> Result<()> {
        test_parse_and_merge(TestSet {
            manifest_yml: r###"sentry: {}"###.into(),
            env_yml: "{}".into(),
            default_channel: "CDEFTEAM8".into(),
            crd_expected: r###"---
sentry:
  slack: CDEFTEAM8
  silent: false"###
                .into(),
        })
    }

    #[test]
    fn test_route_monitoring_to_another_slack_channel() -> Result<()> {
        test_parse_and_merge(TestSet {
            manifest_yml: r###"---
sentry:
  silent: false"###
                .into(),
            env_yml: r###"---
sentry:
  # it's not possible to turn sentry off if defined in the service manifest
  silent: true
  slack: COVERRIDE"###
                .into(),
            default_channel: "CDEFTEAM8".into(),
            crd_expected: r###"---
sentry:
  slack: COVERRIDE
  silent: true"###
                .into(),
        })
    }

    fn test_parse_and_merge(test_set: TestSet) -> Result<()> {
        let manifest: ManifestOverridesNarrowed = serde_yaml::from_str(&test_set.manifest_yml)?;
        let prod: ManifestOverridesNarrowed = serde_yaml::from_str(&test_set.env_yml)?;

        let merge_with_env = manifest.merge(prod);

        println!("merge result:\n{:#?}", merge_with_env);

        let slack_default = &SlackChannel::new(&test_set.default_channel);
        let build_result = merge_with_env.sentry.build(slack_default)?;

        println!("build result:\n{:#?}", build_result);

        println!("EXPECTED:\n{}", test_set.crd_expected);
        let actual = serde_yaml::to_string(&ManifestNarrowed { sentry: build_result })?;
        println!("ACTUAL:\n{}", actual);

        Ok(assert_eq!(test_set.crd_expected.clone(), actual))
    }
}
