use slack_hook::{Slack, PayloadBuilder};
use std::env;

use super::{Result, ErrorKind};


pub struct Message {
    /// Text in message
    pub text: String,

    // Optional link,description pair
    //pub link: Option<(String, String)>
}

// TODO: change errorkind
fn default_hook() -> Result<String> {
    env::var("SLACK_SHIPCAT_HOOK_URL").map_err(|_| ErrorKind::MissingSlackUrl.into())
}
fn default_channel() -> Result<String> {
    env::var("SLACK_SHIPCAT_CHANNEL").map_err(|_| ErrorKind::MissingSlackChannel.into())
}
fn default_username() -> String {
    env::var("SLACK_SHIPCAT_NAME").unwrap_or("shipcat".into())
}

pub fn message(msg: Message) -> Result<()> {
    let hook_url : &str = &default_hook()?;
    let hook_chan : String = default_channel()?;
    let hook_user : String = default_username();

    //let hook_url : String = "https://hooks.slack.com/services/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".into();
    let slack = Slack::new(hook_url).unwrap();
    let p = PayloadBuilder::new()
      .text(msg.text)
      .channel(hook_chan)
      .icon_emoji(":ship:")
      .username(hook_user);

    slack.send(&p.build()?)?;

    Ok(())
}
