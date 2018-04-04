use slack_hook::{Slack, PayloadBuilder, SlackLink, SlackUserLink, AttachmentBuilder};
use slack_hook::SlackTextContent::{Text, Link, User};
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

    /// Optional link to append
    pub link: Option<String>,

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

    // Main link
    if let Some(link) = msg.link {
        let split: Vec<&str> = link.split('|').collect();
        // Full sanity check here as it could come from the CLI
        if split.len() > 2 {
            bail!("Link {} not in the form of url|description", link);
        }
        let desc = if split.len() == 2 { split[1].into() } else { link.clone() };
        let addr = if split.len() == 2 { split[0].into() } else { link.clone() };
        texts.push(Link(SlackLink::new(&addr, &desc)));
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


#[cfg(test)]
mod tests {
    use tests::setup;
    use super::super::{Manifest, Config};
    use super::{send, Message};

    #[test]
    fn slack_test() {
        setup();
        let conf = Config::read().unwrap();
        let mf = Manifest::basic("fake-ask", &conf, Some("dev-uk".into())).unwrap();

        send(Message {
            text: format!("tested {}", "slack"),
            color: Some("good".into()),
            link: Some("https://lolcathost.com/|lolcathost".into()),
            notifies: mf.metadata.contacts,
            code: Some(format!("-diff\n+diff")),
            ..Default::default()
        }).unwrap();
    }
}
