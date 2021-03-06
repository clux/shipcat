mod common;
use crate::common::setup;
use shipcat::slack::{env_channel, send, send_dumb, DumbMessage, Message};
use shipcat_definitions::{structs::NotificationMode, Config, ConfigState};

// integration temporarily disabled
#[tokio::test]
#[ignore]
async fn slack_test() {
    setup();

    let (conf, reg) = Config::new(ConfigState::Base, "dev-uk").await.unwrap();
    let mf = shipcat_filebacked::load_metadata("fake-ask", &conf, &reg)
        .await
        .unwrap();

    let chan = env_channel().unwrap();
    if chan == "#shipcat-test" {
        send_dumb(DumbMessage {
            text: format!("simple `{}` test", "slack"),
            ..Default::default()
        })
        .await
        .unwrap();
        send(
            Message {
                text: format!("Trivial upgrade deploy test of `{}`", "slack"),
                color: Some("good".into()),
                version: mf.version.clone(),
                mode: NotificationMode::default(),
                metadata: mf.base.metadata.clone(),
                code: Some(format!(
                    "Pod changed:
-  image: \"blah:e7c1e5dd5de74b2b5da5eef76eb5bf12bdc2ac19\"
+  image: \"blah:d4f01f5143643e75d9cc2d5e3221e82a9e1c12e5\""
                )),
            },
            &conf.owners,
        )
        .await
        .unwrap();

        // this is not just a three line diff, so
        send(
            Message {
                text: format!("Non-trivial deploy test of `{}`", "slack"),
                color: Some("good".into()),
                mode: NotificationMode::default(),
                metadata: mf.base.metadata,
                version: mf.version.clone(),
                code: Some(format!(
                    "Pod changed:
-  value: \"somedeletedvar\"
-  image: \"blah:abc12345678\"
+  image: \"blah:abc23456789\""
                )),
            },
            &conf.owners,
        )
        .await
        .unwrap();
    }
}
