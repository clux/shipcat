use slack_hook::{Slack, PayloadBuilder, SlackLink};
use slack_hook::SlackTextContent::{Text, Link};
use std::env;

use super::{Result, ErrorKind};


pub struct Message {
    /// Text in message
    pub text: String,

    /// Optional link
    pub link: Option<String>
}

fn env_hook_url() -> Result<String> {
    env::var("SLACK_SHIPCAT_HOOK_URL").map_err(|_| ErrorKind::MissingSlackUrl.into())
}
fn env_channel() -> Result<String> {
    env::var("SLACK_SHIPCAT_CHANNEL").map_err(|_| ErrorKind::MissingSlackChannel.into())
}
fn env_username() -> String {
    env::var("SLACK_SHIPCAT_NAME").unwrap_or("shipcat".into())
}

pub fn message(msg: Message) -> Result<()> {
    let hook_url : &str = &env_hook_url()?;
    let hook_chan : String = env_channel()?;
    let hook_user : String = env_username();

    let slack = Slack::new(hook_url).unwrap();
    let mut p = PayloadBuilder::new().channel(hook_chan)
      .icon_emoji(":ship:")
      .username(hook_user);

    if let Some(link) = msg.link {
        p = p.text(vec![
            Text(msg.text.into()),
            Link(SlackLink::new(&link, &link))
        ].as_slice());
    } else {
        p = p.text(msg.text);
    }
    slack.send(&p.build()?)?;

    Ok(())
}
