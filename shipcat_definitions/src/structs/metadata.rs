use regex::Regex;
use std::ops::{Deref, DerefMut};

use super::Result;
use config::{Team, SlackParameters};

/// Contact data
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Contact {
    /// Free text name
    pub name: String,
    /// Slack handle
    pub slack: String,
    /// Email address
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Github username
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github: Option<String>,
}
impl Contact {
    pub fn verify(&self) -> Result<()> {
        if self.name.is_empty() {
            bail!("Contact name cannot be empty")
        }
        if !self.slack.starts_with("@") {
            bail!("Contact slack handle needs to start with the slack guid '@U...' - got {}", self.slack)
        }
        if self.slack.contains("|") {
            bail!("Contact slack user id invalid - got {}", self.slack)
        }
        if let Some(ref gh) = &self.github {
            if gh.starts_with("@") || gh.contains("/") {
               bail!("github id must be the raw username only - got {}", gh)
            }
            // TODO: check members of org!
        }
        Ok(())
    }
}

/// Slack channel verifier
#[derive(Serialize, Deserialize, PartialEq, Clone, Default, Debug)]
pub struct SlackChannel(String);
impl SlackChannel {
    pub fn new(chan: &str) -> Self {
        SlackChannel(chan.into())
    }

    pub fn verify(&self) -> Result<()> {
        let channelre = Regex::new(r"^#[a-z0-9._-]+$").unwrap();
        if !channelre.is_match(&self.0) {
            bail!("channel is invalid: {}", self.0)
        }

        Ok(())
    }

    // TODO: needs a org id from config
    pub fn link(&self, params: &SlackParameters) -> String {
        let hashless = self.0.clone().split_off(1);
        format!("slack://channel?id={}&team={}", hashless, params.team)
    }
}

impl Deref for SlackChannel {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SlackChannel {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

/// Metadata for a service
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(test, derive(Default))]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    /// Git repository
    pub repo: String,
    /// Owning team
    pub team: String,
    /// Release tagging scheme
    ///
    /// Defaults to the version itself. Leading v tagging services can use "v{{ version }}"
    /// Monorepos that have multiple tags can use "{{ version }}-app"
    #[serde(default = "default_format_string")]
    pub gitTagTemplate: String,
    /// Contact person
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contacts: Vec<Contact>,
    /// Support channel - human interaction
    #[serde(default)]
    pub support: Option<SlackChannel>,
    /// Notifications channel - automated messages
    #[serde(default)]
    pub notifications: Option<SlackChannel>,
    /// Runbook name in repo
    pub runbook: Option<String>,
    /// Canoncal documentation link
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
    // TODO: generate swagger docs url from region and service name
}
fn default_format_string() -> String { "{{ version }}".into() }

impl Metadata {
    pub fn version_template(&self, ver: &str) -> Result<String> {
        use tera::{Tera, Context};
        let mut ctx = Context::new();
        ctx.insert("version", &ver.to_string());
        let res = Tera::one_off(&self.gitTagTemplate, &ctx, false).map_err(|e| {
            warn!("Failed to template gitTagTemplate {}", self.gitTagTemplate);
            e
        })?;
        Ok(res)
    }
}

impl Metadata {
    pub fn verify(&self, teams: &[Team]) -> Result<()> {
        let ts = teams.to_vec().into_iter().map(|t| t.name).collect::<Vec<_>>();
        if !ts.contains(&self.team) {
            bail!("Illegal team name {} not found in the config", self.team);
        }
        for cc in &self.contacts {
            cc.verify()?;
        }
        let re = Regex::new(r"[a-z0-9\-\.\{\}]").unwrap();
        if !re.is_match(&self.gitTagTemplate) {
            bail!("gitTagTemplate {} is of invalid format", self.gitTagTemplate);
        }
        let sanityre = Regex::new(r"\{\{.?version.?\}\}").unwrap();
        if !sanityre.is_match(&self.gitTagTemplate) {
            bail!("gitTagTemplate {} does not dereference {{ version }}", self.gitTagTemplate);
        }
        if let Some(channel) = &self.support {
            channel.verify()?;
        }
        if let Some(channel) = &self.notifications {
            channel.verify()?;
        }
        if let Some(runbook) = &self.runbook {
            if !runbook.ends_with(".md") && !runbook.ends_with(".rt") {
                bail!("Runbook must be in markdown or restructured text in the service repo");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Metadata;
    use super::SlackChannel;
    use super::default_format_string;

    #[test]
    fn version_tpl() {
        // a metadata struct that simulates a serde parsed with missing tag template
        let mut md = Metadata {
            gitTagTemplate: default_format_string(),
            ..Default::default()
        };
        let defres = md.version_template("1.2.3");
        assert!(defres.is_ok());
        assert_eq!(defres.unwrap(), "1.2.3");

        md.gitTagTemplate = "prefix-{{ version }}-suffix".to_string();
        let res = md.version_template("0.1.2");
        assert!(res.is_ok());
        let ru = res.unwrap();
        assert_eq!(ru, "prefix-0.1.2-suffix")
    }

    #[test]
    fn valid_slack_channel() {
        let sc = SlackChannel::new("#dev-platform");
        let valid = sc.verify();
        println!("{:?}", valid);
        assert!(valid.is_ok());
    }

    #[test]
    fn invalid_slack_channel() {
        let sc = SlackChannel::new("# iaminvalidåß∂ƒ••");
        let valid = sc.verify();
        println!("{:?}", valid);
        assert!(valid.is_err());
    }
}
