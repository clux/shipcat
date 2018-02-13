use slack_hook::{Slack, PayloadBuilder, SlackLink, AttachmentBuilder};
use slack_hook::SlackTextContent::{Text, Link};
use std::env;

use super::{Result, ErrorKind};


pub struct Message {
    /// Text in message
    pub text: String,

    /// Optional link
    pub link: Option<String>,

    /// Color
    pub color: Option<String>,

    /// Reason
    pub reason: Option<String>,
}

fn env_hook_url() -> Result<String> {
    env::var("SLACK_SHIPCAT_HOOK_URL").map_err(|_| ErrorKind::MissingSlackUrl.into())
}
fn env_channel() -> Result<String> {
    env::var("SLACK_SHIPCAT_CHANNEL").map_err(|_| ErrorKind::MissingSlackChannel.into())
}
fn env_username() -> String {
    env::var("SLACK_SHIPCAT_NAME").unwrap_or_else(|_| "shipcat".into())
}

pub fn message(msg: Message) -> Result<()> {
    let hook_url : &str = &env_hook_url()?;
    let hook_chan : String = env_channel()?;
    let hook_user : String = env_username();
    // TODO: check hook url non-empty?

    let slack = Slack::new(hook_url).unwrap();
    let mut p = PayloadBuilder::new().channel(hook_chan)
      .icon_emoji(":ship:")
      .username(hook_user);

    if let Some(link) = msg.link {
        let split: Vec<&str> = link.split('|').collect();
        if split.len() > 2 {
            bail!("Link {} not in the form of url|description", link);
        }
        let desc = if split.len() == 2 { split[1].into() } else { link.clone() };
        let addr = if split.len() == 2 { split[0].into() } else { link.clone() };
        // TODO: allow multiple links!
        p = p.text(vec![
            Text(msg.text.into()),
            Link(SlackLink::new(&addr, &desc))
        ].as_slice());
    } else {
        p = p.text(msg.text);
    }
    if let Some(r) = msg.reason {
        let mut a = AttachmentBuilder::new(r);
        if let Some(c) = msg.color {
            a = a.color(c)
        }
        p = p.attachments(vec![a.build()?]);
    }
    slack.send(&p.build()?)?;

    Ok(())
}
