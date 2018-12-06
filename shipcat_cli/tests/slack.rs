mod common;
use crate::common::setup;

extern crate shipcat;
extern crate shipcat_definitions;

use shipcat::{Manifest};
use shipcat::slack::{send, Message, env_channel};

#[test]
fn slack_test() {
    setup();
    // metadata is global - can use a blank manifest for this test
    let mf = Manifest::blank("fake-ask").unwrap();

    let chan = env_channel().unwrap();
    if chan == "#shipcat-test" {
      send(Message {
          text: format!("simple `{}` test", "slack"),
          ..Default::default()
      }).unwrap();
      send(Message {
            text: format!("Trivial upgrade deploy test of `{}`", "slack"),
            color: Some("good".into()),
            metadata: mf.metadata.clone(),
            code: Some(format!("Pod changed:
-  image: \"blah:e7c1e5dd5de74b2b5da5eef76eb5bf12bdc2ac19\"
+  image: \"blah:d4f01f5143643e75d9cc2d5e3221e82a9e1c12e5\"")),
            ..Default::default()
        }).unwrap();
      // this is not just a three line diff, so
      send(Message {
            text: format!("Non-trivial deploy test of `{}`", "slack"),
            color: Some("good".into()),
            metadata: mf.metadata,
            code: Some(format!("Pod changed:
-  value: \"somedeletedvar\"
-  image: \"blah:abc12345678\"
+  image: \"blah:abc23456789\"")),
            ..Default::default()
        }).unwrap();
    }
}
