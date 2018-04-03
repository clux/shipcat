use slack_hook::{Slack, PayloadBuilder, Parse, SlackLink, AttachmentBuilder};
use slack_hook::SlackTextContent::{Text, Link};
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
      .username(hook_user)
      .link_names(true); // seems to only do it for long usernames..

    let mut a = AttachmentBuilder::new(msg.text.clone());
    if let Some(c) = msg.color {
        a = a.color(c)
    }

    if let Some(link) = msg.link {
        let split: Vec<&str> = link.split('|').collect();
        if split.len() > 2 {
            bail!("Link {} not in the form of url|description", link);
        }
        let desc = if split.len() == 2 { split[1].into() } else { link.clone() };
        let addr = if split.len() == 2 { split[0].into() } else { link.clone() };
        // TODO: allow multiple links!
        //a = a.title_link(&addr as &str).title(desc).text(msg.text);

        a = a.text(vec![
            Text(msg.text.into()),
            Link(SlackLink::new(&addr, &desc)),
        ].as_slice());
    } else {
        a = a.text(msg.text);
    }
    let mut ax = vec![a.build()?];
    if let Some(code) = msg.code {
        let a2 = AttachmentBuilder::new(code.clone())
            .color("#439FE0")
            .text(vec![Text(code.into())].as_slice())
            .build()?;
        ax.push(a2);
    }
    p = p.attachments(ax);

    slack.send(&p.build()?)?;

    Ok(())
}


#[cfg(test)]
mod tests {
    use tests::setup;
    use super::{send, Message};

    #[test]
    fn slack_test() {
        setup();
        let ccs = format!("@clux @florent wtf");
        send(Message {
            text: format!("tested {} {} ", "slack", ccs),
            color: Some("good".into()),
            link: Some("https://jenkins.blah/job/testjob/1792/|testjob #1792".into()),
            code: Some(format!("-diff\n+diff")),
            ..Default::default()
        }).unwrap();
}
}
