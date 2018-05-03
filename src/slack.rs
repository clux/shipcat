use slack_hook::{Slack, PayloadBuilder, SlackLink, SlackText, SlackUserLink, AttachmentBuilder};
use slack_hook::SlackTextContent::{self, Text, Link, User};
use std::env;

use super::{Result, ErrorKind};

/// Slack message options we support
///
/// These parameters get distilled into the attachments API.
/// Mostly because this is the only thing API that supports colour.
#[derive(Default)]
pub struct Message {
    /// Text in message
    pub text: String,

    /// Optional list of people links to CC
    pub notifies: Vec<String>,

    /// Optional color for the attachment API
    pub color: Option<String>,

    /// Optional code input
    pub code: Option<String>,
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
    // TODO: check hook url non-empty?

    let slack = Slack::new(hook_url).unwrap();
    let mut p = PayloadBuilder::new().channel(hook_chan)
      .icon_emoji(":ship:")
      .username(hook_user);

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

    // Auto link/text from originator
    if let Some(orig) = infer_ci_links() {
        texts.push(orig);
    }

    // CCs from notifies array
    for cc in msg.notifies {
        texts.push(User(SlackUserLink::new(&cc)));
    }

    // Pass the texts array to slack_hook
    a = a.text(texts.as_slice());

    // Second attachment: optional code (blue)
    let mut ax = vec![a.build()?];
    if let Some(code) = msg.code {
        let a2 = AttachmentBuilder::new(code.clone())
            .color("#439FE0")
            .text(vec![Text(code.into())].as_slice())
            .build()?;
        ax.push(a2);
    }

    // Pass attachment vector
    p = p.attachments(ax);
    // Send everything. Phew.
    slack.send(&p.build()?)?;

    Ok(())
}

/// Infer originator of a message
fn infer_ci_links() -> Option<SlackTextContent> {
    use std::env;
    use std::process::Command;
    if let (Ok(url), Ok(name), Ok(nr)) = (env::var("BUILD_URL"),
                                          env::var("JOB_NAME"),
                                          env::var("BUILD_NUMBER")) {
        // we are on jenkins
        Some(Link(SlackLink::new(&url, &format!("{} #{}", name, nr))))
    } else if let (Ok(url), Ok(name), Ok(nr)) = (env::var("CIRCLE_BUILD_URL"),
                                                 env::var("CIRCLE_JOB"),
                                                 env::var("CIRCLE_BUILD_NUM")) {
        // we are on circle
        Some(Link(SlackLink::new(&url, &format!("{} #{}", name, nr))))
    } else {
        // fallback to linux user
        match Command::new("whoami").output() {
            Ok(s) => {
                let mut out : String = String::from_utf8_lossy(&s.stdout).into();
                let len = out.len();
                if out.ends_with('\n') {
                    out.truncate(len - 1)
                }
                Some(Text(SlackText::new(format!("({})", out))))
            }
            Err(e) => {
                warn!("Could not retrieve user from shell {}", e);
                None
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use tests::setup;
    use super::super::{Manifest, Config};
    use super::{send, Message, env_channel};

    #[test]
    fn slack_test() {
        setup();
        let conf = Config::read().unwrap();
        let mf = Manifest::basic("fake-ask", &conf, Some("dev-uk".into())).unwrap();

        let chan = env_channel().unwrap();
        if chan == "#shipcat-test" {
            send(Message {
                text: format!("tested `{}`", "slack"),
                color: Some("good".into()),
                notifies: mf.metadata.unwrap().contacts,
                code: Some(format!("-diff\n+diff")),
                ..Default::default()
            }).unwrap();
        }
    }
}
