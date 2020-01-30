use regex::Regex;
use std::collections::BTreeMap;

use merge::Merge;
use shipcat_definitions::structs::metadata::SlackChannel;
use shipcat_definitions::structs::newrelic::{Newrelic, NewrelicAlert, NewrelicIncidentPreference};
use shipcat_definitions::{Result, ResultExt};

use crate::util::Build;

/// Monitoring section covering Newrelic configuration
///
/// ```yaml
/// newrelic:
///   slack: C12ABYZ78
///   incidentPreference: PER_POLICY
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
    /// we might want to re-route all the alerts to some particular channel in bulk
    pub slack: Option<SlackChannel>,
    #[serde(default)]
    pub incident_preference: Option<NewrelicIncidentPreference>,
    #[serde(default)]
    pub alerts: BTreeMap<String, Option<NewrelicAlertSource>>,
}

impl Build<Option<Newrelic>, SlackChannel> for NewrelicSource {
    fn build(self, default_channel: &SlackChannel) -> Result<Option<Newrelic>> {
        let slack: SlackChannel = self
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
                        .build(&name.clone())
                        .map(|alert| (name.clone(), alert))
                        .chain_err(|| format!("Error at Newrelic Alert `{}`", &name))
                })
            })
            .collect();

        let alerts: BTreeMap<String, NewrelicAlert> = alerts_res?;

        if alerts.len() > 0 && !slack.starts_with("C") {
            bail!(
                "Private/personal channel {} is NOT supported as NewRelic target",
                *slack
            );
        }

        let incident_preference = self.incident_preference.unwrap_or_default();

        Ok(Some(Newrelic {
            slack,
            incident_preference,
            alerts,
        }))
    }
}

#[derive(Debug, Default, Clone, Deserialize, Merge)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NewrelicAlertSource {
    pub template: Option<String>,
    #[serde(default)]
    pub params: BTreeMap<String, String>,
}

impl Build<NewrelicAlert, String> for NewrelicAlertSource {
    fn build(self, name: &String) -> Result<NewrelicAlert> {
        let name_re = Regex::new(r"^[0-9a-zA-Z _/\-]{1,50}$").unwrap();
        if !name_re.is_match(name) {
            bail!(
                "Only alnums, '_', '/', '-' allowed for alert name, which is `{}`",
                name
            );
        }

        let template = self.template.unwrap_or(name.to_string());
        let template_re = Regex::new(r"^[0-9a-zA-Z_/\-]{1,50}$").unwrap();
        if !template_re.is_match(&template) {
            bail!(
                "Only alnums, '_', '/', '-' allowed for template name, which is `{}` for alert `{}`",
                template,
                name
            );
        }

        Ok(NewrelicAlert {
            name: name.to_string(),
            template,
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
            crd_expected: "---\n{}".into(),
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
      template: default
    thisOneHasSlackDefinedAndWontPropagate:
      template: default"###
                .into(),
            env_yml: "{}".into(),
            default_channel: "CDEFTEAM8".into(),
            crd_expected: r###"---
newrelic:
  alerts:
    allParamsAreDefault:
      name: allParamsAreDefault
      template: default
      params: {}
    thisOneHasSlackDefinedAndWontPropagate:
      name: thisOneHasSlackDefinedAndWontPropagate
      template: default
      params: {}
  incidentPreference: PER_POLICY
  slack: CDEFTEAM8"###
                .into(),
        })
    }

    #[test]
    fn test_template_name_defaults_to_alert_name() -> Result<()> {
        test_parse_and_merge(TestSet {
            manifest_yml: r###"---
newrelic:
  alerts:
    allPara/msAreDefault: {}
    thisOneHasTemplateDefinedAndWontPropagate:
      template: some/subfolder/default"###
                .into(),
            env_yml: "{}".into(),
            default_channel: "CDEFTEAM8".into(),
            crd_expected: r###"---
newrelic:
  alerts:
    allPara/msAreDefault:
      name: allPara/msAreDefault
      template: allPara/msAreDefault
      params: {}
    thisOneHasTemplateDefinedAndWontPropagate:
      name: thisOneHasTemplateDefinedAndWontPropagate
      template: some/subfolder/default
      params: {}
  incidentPreference: PER_POLICY
  slack: CDEFTEAM8"###
                .into(),
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
      template: Default"###
                .into(),
            env_yml: r###"---
newrelic:
  slack: CREGION78"###
                .into(),
            default_channel: "CDEFTEAM8".into(),
            crd_expected: r###"---
newrelic:
  alerts:
    allParamsAreDefault:
      name: allParamsAreDefault
      template: Default
      params: {}
    thisOneHasSlackDefinedAndWontChange:
      name: thisOneHasSlackDefinedAndWontChange
      template: Default
      params: {}
  incidentPreference: PER_POLICY
  slack: CREGION78"###
                .into(),
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
        priority: warning"###
                .into(),
            env_yml: r###"---
newrelic:
  slack: COVERRIDE
  incidentPreference: PER_CONDITION_AND_TARGET
  alerts:
    myApdex: # you have to specify the full alert body, as it's merged as a whole
      template: SimpleApdex
      params:
        threshold: "0.98"
        priority: warning
    myErrorRate:
      template: SimpleErrorRate
      params:
        threshold: "0.05"
        priority: critical"###
                .into(),
            default_channel: "CDEFTEAM8".into(),
            crd_expected: r###"---
newrelic:
  alerts:
    myApdex:
      name: myApdex
      template: SimpleApdex
      params:
        priority: warning
        threshold: "0.98"
    myErrorRate:
      name: myErrorRate
      template: SimpleErrorRate
      params:
        priority: critical
        threshold: "0.05"
  incidentPreference: PER_CONDITION_AND_TARGET
  slack: COVERRIDE"###
                .into(),
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
        priority: warning"###
                .into(),
            env_yml: r###"---
newrelic:
  alerts:
    myErrorRate: ~ # kill an alert by mapping it to null value"###
                .into(),
            default_channel: "CDEFTEAM8".into(),
            crd_expected: r###"---
newrelic:
  alerts:
    myApdex:
      name: myApdex
      template: SimpleApdex
      params:
        priority: warning
        threshold: "0.8"
  incidentPreference: PER_POLICY
  slack: CDEFTEAM8"###
                .into(),
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
