use regex::Regex;
use std::collections::BTreeMap;

use merge::Merge;
use shipcat_definitions::structs::metadata::SlackChannel;
use shipcat_definitions::structs::newrelic::{Newrelic, NewrelicAlert};
use shipcat_definitions::{Result, ResultExt};

use crate::util::Build;

/// Monitoring section covering Newrelic configuration
///
/// ```yaml
/// newrelic:
///   slack: C12ABYZ78
///   alerts:
///     my_alert_name_foo:
///       template: apdex
///       enabled: true
///       # your alert-level override for slack target
///       slack: C1DEVOPS8
///       params:
///         duration: 60
///         threshold: 0.5
/// ```
#[derive(Debug, Default, Clone, Deserialize, Merge)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NewrelicSource {
    /// we might want to route all the alerts to some particular channel in bulk
    /// without having to mention each alert explicitly
    pub slack: Option<SlackChannel>,
    #[serde(default)]
    pub alerts: BTreeMap<String, Option<NewrelicAlertSource>>,
}

impl Build<Option<Newrelic>, SlackChannel> for NewrelicSource {
    fn build(self, default_channel: &SlackChannel) -> Result<Option<Newrelic>> {
        let slack = self
            .slack
            .map(|s| s.verify().map(|_| s))
            .unwrap_or_else(|| Ok(default_channel.clone()))?;

        if self.alerts.is_empty() {
            return Ok(None);
        }

        let alerts_res: Result<BTreeMap<String, NewrelicAlert>> = self
            .alerts
            .into_iter()
            .filter_map(|(name, opt_alert_source)| {
                opt_alert_source.map(|alert_source| {
                    alert_source
                        .build(&(name.clone(), slack.clone()))
                        .map(|alert| (name.clone(), alert))
                        .chain_err(|| format!("Error at Newrelic Alert `{}`", &name))
                })
            })
            .collect();

        Ok(Some(Newrelic { alerts: alerts_res? }))
    }
}

#[derive(Debug, Default, Clone, Deserialize, Merge)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NewrelicAlertSource {
    pub template: Option<String>,
    pub slack: Option<SlackChannel>,
    #[serde(default)]
    pub params: BTreeMap<String, String>,
}

impl Build<NewrelicAlert, (String, SlackChannel)> for NewrelicAlertSource {
    fn build(self, name_slack: &(String, SlackChannel)) -> Result<NewrelicAlert> {
        let (name, default_channel) = name_slack;
        let slack = self
            .slack
            .map(|s| s.verify().map(|_| s))
            .unwrap_or_else(|| Ok(default_channel.clone()))?;

        let name_re = Regex::new(r"^[0-9a-zA-Z _\-]{1,50}$").unwrap();
        if !name_re.is_match(name) {
            bail!(
                "Alnums, dashes, underscores and spaces only for alert name, which is `{}`",
                name
            );
        }

        let template = self
            .template
            .ok_or(format!("Template name for alert `{}` must be provided", name))?;
        let template_re = Regex::new(r"^[0-9a-zA-Z_\-]{1,50}$").unwrap();
        if !template_re.is_match(&template) {
            bail!(
                "Alnums, dashes, underscores only for template name, which is `{}` for alert `{}`",
                template,
                name
            );
        }

        Ok(NewrelicAlert {
            name: name.to_string(),
            template,
            slack,
            params: self.params,
        })
    }
}

#[cfg(test)]
mod tests {
    use merge::Merge;
    use shipcat_definitions::structs::metadata::SlackChannel;
    use shipcat_definitions::structs::newrelic::Newrelic;
    use shipcat_definitions::Result;

    use super::super::util::Build;
    use super::NewrelicSource;

    //  make sure the macros are called as they are for actual/original structs
    #[derive(Deserialize, Default, Merge, Clone, Debug)]
    #[serde(default, deny_unknown_fields, rename_all = "camelCase")]
    pub struct ManifestOverridesNarrowed {
        #[serde(default)]
        newrelic: NewrelicSource,
    }

    #[derive(Serialize, Deserialize, Clone, Default)]
    pub struct ManifestNarrowed {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub newrelic: Option<Newrelic>,
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
{}"###.into(),
        })
    }

    #[test]
    fn test_team_default_slack_propagated() -> Result<()> {
        //  plus coverage for 0-params newrelic template - {} should be included
        test_parse_and_merge(TestSet {
            manifest_yml: r###"---
newrelic:
  alerts:
    allParamsAreDefault:
      template: Default
    thisOneHasSlackDefinedAndWontPropagate:
      slack: COVERRIDE
      template: Default"###.into(),
            env_yml: "{}".into(),
            default_channel: "CDEFTEAM8".into(),
            crd_expected: r###"---
newrelic:
  alerts:
    allParamsAreDefault:
      name: allParamsAreDefault
      template: Default
      slack: CDEFTEAM8
      params: {}
    thisOneHasSlackDefinedAndWontPropagate:
      name: thisOneHasSlackDefinedAndWontPropagate
      template: Default
      slack: COVERRIDE
      params: {}"###.into(),
        })
    }

    #[test]
    fn test_route_region_alerts_to_another_channel() -> Result<()> {
        test_parse_and_merge(TestSet {
            manifest_yml: r###"---
newrelic:
  alerts:
    allParamsAreDefault:
      template: Default
    thisOneHasSlackDefinedAndWontChange:
      slack: COVERRIDE
      template: Default"###.into(),
            env_yml: r###"---
newrelic:
  slack: CREGION78"###.into(),
            default_channel: "CDEFTEAM8".into(),
            crd_expected: r###"---
newrelic:
  alerts:
    allParamsAreDefault:
      name: allParamsAreDefault
      template: Default
      slack: CREGION78
      params: {}
    thisOneHasSlackDefinedAndWontChange:
      name: thisOneHasSlackDefinedAndWontChange
      template: Default
      slack: COVERRIDE
      params: {}"###.into(),
        })
    }

    #[test]
    fn test_tighten_prod_thresholds_and_levels() -> Result<()> {
        test_parse_and_merge(TestSet {
            manifest_yml: r###"---
newrelic:
  alerts:
    myApdex:
      template: SimpleApdex
      params:
        threshold: "0.8"
        priority: warning
    myErrorRate:
      template: SimpleErrorRate
      params:
        threshold: "0.05"
        priority: warning"###.into(),
            env_yml: r###"---
newrelic:
  alerts:
    myApdex: # you have to specify the full alert body, as it's merged as a whole
      template: SimpleApdex
      params:
        threshold: "0.98"
        priority: warning
    myErrorRate:
      slack: COVERRIDE
      template: SimpleErrorRate
      params:
        threshold: "0.05"
        priority: critical"###.into(),
            default_channel: "CDEFTEAM8".into(),
            crd_expected: r###"---
newrelic:
  alerts:
    myApdex:
      name: myApdex
      template: SimpleApdex
      slack: CDEFTEAM8
      params:
        priority: warning
        threshold: "0.98"
    myErrorRate:
      name: myErrorRate
      template: SimpleErrorRate
      slack: COVERRIDE
      params:
        priority: critical
        threshold: "0.05""###.into(),
        })
    }

    #[test]
    fn test_turn_alerts_off_for_dev() -> Result<()> {
        test_parse_and_merge(TestSet {
            manifest_yml: r###"---
newrelic:
  alerts:
    myApdex:
      template: SimpleApdex
      params:
        threshold: "0.8"
        priority: warning
    myErrorRate:
      template: SimpleErrorRate
      params:
        threshold: "0.05"
        priority: warning"###.into(),
            env_yml: r###"---
newrelic:
  alerts:
    myErrorRate: ~ # kill an alert by mapping it to null value"###.into(),
            default_channel: "CDEFTEAM8".into(),
            crd_expected: r###"---
newrelic:
  alerts:
    myApdex:
      name: myApdex
      template: SimpleApdex
      slack: CDEFTEAM8
      params:
        priority: warning
        threshold: "0.8""###.into(),
        })
    }

    fn test_parse_and_merge(test_set: TestSet) -> Result<()> {
        let manifest: ManifestOverridesNarrowed = serde_yaml::from_str(&test_set.manifest_yml)?;
        let prod: ManifestOverridesNarrowed = serde_yaml::from_str(&test_set.env_yml)?;

        let merge_with_env = manifest.merge(prod);

        println!("merge result:\n{:#?}", merge_with_env);

        let slack_default = &SlackChannel::new(&test_set.default_channel);
        let build_result = merge_with_env.newrelic.build(slack_default)?;

        println!("build result:\n{:#?}", build_result);

        println!("EXPECTED:\n{}", test_set.crd_expected);
        let actual = serde_yaml::to_string(&ManifestNarrowed {
            newrelic: build_result,
        })?;
        println!("ACTUAL:\n{}", actual);

        Ok(assert_eq!(test_set.crd_expected.clone(), actual))
    }
}
