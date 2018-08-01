///
/// Interface for adding grafana annotations about deploys
///

use reqwest;
use hyper::header::{Headers, Authorization, Bearer, ContentType};
use super::{Result, ErrorKind, ResultExt};
use chrono::Utc;
use std::env;

/// At what time the annotation should be made
#[derive(Debug)]
pub enum TimeSpec {
    Now,
    Time(u64),
}

/// The type of annotation event
#[derive(Debug)]
pub enum Event {
    Upgrade,
    Rollback,
}

/// A representation of a particular deployment event
#[derive(Debug)]
pub struct Annotation {
    pub event: Event,
    pub service: String,
    pub version: String,
    pub region: String,
    pub time: TimeSpec,
}

/// Extracts grafana URL + HTTP scheme from environment
pub fn env_hook_url() -> Result<String> {
    env::var("GRAFANA_SHIPCAT_HOOK_URL")
        .map_err(|_| ErrorKind::MissingGrafanaUrl.into())
}

/// Extracts grafana API key from environment
pub fn env_token() -> Result<String> {
    env::var("GRAFANA_SHIPCAT_TOKEN")
        .map_err(|_| ErrorKind::MissingGrafanaToken.into())
}

/// Convert timespec to UNIX time, in milliseconds
fn unix_timestamp(spec: &TimeSpec) -> Result<u64> {
  let timestamp = match spec {
    TimeSpec::Now => Utc::now().timestamp_millis() as u64,
    TimeSpec::Time(timestamp) => *timestamp
  };
  Ok(timestamp)
}

/// Create an annotation for a deployment using grafana's REST API
pub fn create(annotation: Annotation) -> Result<()> {
    let hook_url = env_hook_url()?;
    let hook_token = env_token()?;
    
    let timestamp = unix_timestamp(&annotation.time)?;

    let body = json!({
        "time": timestamp,
        "text": format!("{} {}={} in {}",
            match annotation.event {
                Event::Upgrade => "Upgrade",
                Event::Rollback => "Rollback"
            },
            &annotation.service,
            &annotation.version,
            &annotation.region
        ),
        "tags": [
            "all-deploys",
            format!("{}-deploys", annotation.region),
            format!("{}-deploys", annotation.service)
        ]
    });

    let mut headers = Headers::new();
    headers.set(
        Authorization(Bearer { token: hook_token })
    );
    headers.set(
        ContentType("application/json".parse().unwrap())
    );

    let url = reqwest::Url::parse(&hook_url)?.join("api/annotations")?;
    let mkerr = || ErrorKind::Url(url.clone());
    let client = reqwest::Client::new();

    client.post(url.clone())
        .headers(headers)
        .body(body.to_string())
        .send()
        .chain_err(&mkerr)?;
    Ok(())
}
