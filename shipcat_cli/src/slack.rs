use slack_hook::{Slack, PayloadBuilder, SlackLink, SlackText, SlackUserLink, AttachmentBuilder};
use slack_hook::SlackTextContent::{self, Text, Link, User};
use std::env;
use semver::Version;

use super::helm::helpers;
use super::structs::Metadata;
use super::{Result, ErrorKind, ResultExt};

/// Slack message options we support
///
/// These parameters get distilled into the attachments API.
/// Mostly because this is the only thing API that supports colour.
#[derive(Default, Debug)]
pub struct Message {
    /// Text in message
    pub text: String,

    /// Metadata from Manifest
    pub metadata: Option<Metadata>,

    /// Set when not wanting to niotify people
    pub quiet: bool,

    /// Replacement link for CI infer
    pub link: Option<String>,

    /// Optional color for the attachment API
    pub color: Option<String>,

    /// Optional code input
    pub code: Option<String>,

    /// Optional version to send when not having code diffs
    pub version: Option<String>,
}

pub fn env_hook_url() -> Result<String> {
    env::var("SLACK_SHIPCAT_HOOK_URL").map_err(|_| ErrorKind::MissingSlackUrl.into())
}
pub fn env_channel() -> Result<String> {
    env::var("SLACK_SHIPCAT_CHANNEL").map_err(|_| ErrorKind::MissingSlackChannel.into())
}
fn env_username() -> String {
    env::var("SLACK_SHIPCAT_NAME").unwrap_or_else(|_| "shipcat".into())
}

/// Basic check to see that slack credentials is working
///
/// Used before running upgrades so we have a trail
/// It's not very good at the moment. TODO: verify better
pub fn have_credentials() -> Result<()> {
    env_channel()?;
    env_hook_url()?;
    Ok(())
}

/// Send a `Message` to a configured slack destination
pub fn send(msg: Message) -> Result<()> {
    let hook_url : &str = &env_hook_url()?;
    let hook_chan : String = env_channel()?;
    let hook_user : String = env_username();

    // if hook url is invalid, chain it so we know where it came from:
    let slack = Slack::new(hook_url).chain_err(|| ErrorKind::SlackSendFailure(hook_url.to_string()))?;
    let mut p = PayloadBuilder::new().channel(hook_chan)
      .icon_emoji(":ship:")
      .username(hook_user);

    debug!("Got slack notify {:?}", msg);
    // NB: cannot use .link_names due to https://api.slack.com/changelog/2017-09-the-one-about-usernames
    // NB: cannot use .parse(Parse::Full) as this breaks the other links
    // Thus we have to use full slack names, and construct SlackLink objs manually

    // All text is in either one or two attachments to make output as clean as possible

    // First attachment is main text + main link + CCs
    // Fallbacktext is in constructor here (shown in OSD notifies)
    let mut a = AttachmentBuilder::new(msg.text.clone()); // <- fallback
    if let Some(c) = msg.color {
        a = a.color(c)
    }
    // All text constructed for first attachment goes in this vec:
    let mut texts = vec![Text(msg.text.into())];

    if msg.code.is_some() && msg.metadata.is_none() {
        // TODO: only use this when notifying internally
        warn!("Not providing a slack github link due to missing metadata in manifest");
    }

    let mut codeattach = None;
    if let Some(diff) = msg.code {
        // metadata always exists by Manifest::verify
        let md = msg.metadata.clone().unwrap();
        // does the diff contain versions?
        let mut diff_is_pure_verison_change = false;
        if let Some((v1, v2)) = helpers::infer_version_change(&diff) {
            let lnk = create_github_compare_url(&md, (&v1, &v2));
            diff_is_pure_verison_change = helpers::diff_is_version_only(&diff, (&v1, &v2));
            texts.push(lnk);
        }
        // attach full diff as a slack attachment otherwise
        if !diff_is_pure_verison_change {
            codeattach = Some(AttachmentBuilder::new(diff.clone())
                .color("#439FE0")
                .text(vec![Text(diff.into())].as_slice())
                .build()?)
        }
    } else if let Some(v) = msg.version {
        if let Some(ref md) = msg.metadata {
           texts.push(infer_metadata_single_link(md, v));
        }
    }

    if let Some(link) = msg.link {
        let split: Vec<&str> = link.split('|').collect();
        // Full sanity check here as it could come from the CLI
        if split.len() > 2 {
            bail!("Link {} not in the form of url|description", link);
        }
        let desc = if split.len() == 2 { split[1].into() } else { link.clone() };
        let addr = if split.len() == 2 { split[0].into() } else { link.clone() };
        texts.push(Link(SlackLink::new(&addr, &desc)));
    } else {
        // Auto link/text from originator if no ink set
        texts.push(infer_ci_links());
    }

    // Auto cc users
    if let Some(ref md) = msg.metadata {
        if !msg.quiet {
            texts.push(Text("<- ".to_string().into()));
            texts.extend(infer_slack_notifies(md));
        }
    }

    // Pass the texts array to slack_hook
    a = a.text(texts.as_slice());
    let mut ax = vec![a.build()?];

    // Second attachment: optional code (blue)
    if let Some(diffattach) = codeattach {
        ax.push(diffattach);
        // Pass attachment vector

    }
    p = p.attachments(ax);

    // Send everything. Phew.
    slack.send(&p.build()?).chain_err(|| ErrorKind::SlackSendFailure(hook_url.to_string()))?;

    Ok(())
}

fn short_ver(ver: &str) -> String {
    if Version::parse(&ver).is_err() && ver.len() == 40 {
        // only abbreviate versions that are not semver and 40 chars (git shas)
        format!("{}", &ver[..8])
    } else {
        ver.to_string()
    }
}

fn infer_metadata_single_link(md: &Metadata, ver: String) -> SlackTextContent {
    let url = if Version::parse(&ver).is_ok() {
        let tag = md.version_template(&ver).unwrap_or(ver.to_string());
        format!("{}/releases/tag/{}", md.repo, tag)
    } else {
        format!("{}/commit/{}", md.repo, ver)
    };
    Link(SlackLink::new(&url, &short_ver(&ver)))
}

fn create_github_compare_url(md: &Metadata, vers: (&str, &str)) -> SlackTextContent {
    let (v0, v1) = if Version::parse(vers.0).is_ok() {
        let v0 = md.version_template(&vers.0).unwrap_or(vers.0.to_string());
        let v1 = md.version_template(&vers.1).unwrap_or(vers.1.to_string());
        (v0, v1)
    } else {
        (vers.0.into(), vers.1.into())
    };
    let url = format!("{}/compare/{}...{}", md.repo, v0, v1);
    Link(SlackLink::new(&url, &short_ver(vers.1)))
}

fn infer_slack_notifies(md: &Metadata) -> Vec<SlackTextContent> {
    md.contacts.iter().map(|cc| { User(SlackUserLink::new(&cc.slack)) }).collect()
}

/// Infer originator of a message
fn infer_ci_links() -> SlackTextContent {
    if let (Ok(url), Ok(name), Ok(nr)) = (env::var("BUILD_URL"),
                                          env::var("JOB_NAME"),
                                          env::var("BUILD_NUMBER")) {
        // we are on jenkins
        Link(SlackLink::new(&url, &format!("{}#{}", name, nr)))
    } else if let (Ok(url), Ok(name), Ok(nr)) = (env::var("CIRCLE_BUILD_URL"),
                                                 env::var("CIRCLE_JOB"),
                                                 env::var("CIRCLE_BUILD_NUM")) {
        // we are on circle
        Link(SlackLink::new(&url, &format!("{}#{}", name, nr)))
    } else if let Ok(user) = env::var("USER") {
        Text(SlackText::new(format!("(via {})", user)))
    } else {
        warn!("Could not infer ci links from environment");
        Text(SlackText::new("via unknown user".to_string()))
    }
}
