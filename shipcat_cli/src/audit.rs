use chrono::{Utc, SecondsFormat};

use super::{Result, ResultExt, ErrorKind};
use super::AuditWebhook;
use helm::direct::{UpgradeData, UpgradeState};

/// Payload that gets sent via audit webhook
#[derive(Serialize, Clone, Default, Debug)]
#[serde(rename_all = "snake_case")]
struct AuditDeploymentPayload {
    #[serde(rename = "type")]
    pub domainType: String,
    pub timestamp: String,
    pub status: UpgradeState,
    pub upstreamId: Option<String>,
    pub deployment: AuditDeployment,
}

impl AuditDeploymentPayload {
    /// Timestamped payload skeleton
    pub fn new(dt: &str, us: &UpgradeState, ui: &Option<String>) -> Self {
        AuditDeploymentPayload{
            domainType: String::from(dt),
            timestamp: format!("{}", Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)),
            status: us.clone(),
            upstreamId: ui.clone(),
            ..Default::default()
        }
    }
}

/// Payload for single deployment domain object
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
struct AuditDeployment {
    pub id: String,
    pub region: String,
    pub revisionId: String,
    pub service: String,
    pub version: String,
}

impl AuditDeployment {
    pub fn new(udopt: &Option<UpgradeData>, ) -> Self {
        let (service, region, version) = if let Some(ref ud) = udopt {
            (ud.name.clone(), ud.region.clone(), ud.version.clone())
        } else {
            ("unknown".into(), "unknown".into(), "unknown".into())
        };
        let rid = "todo".into();

        AuditDeployment{
            id: format!("{}-{}-{}", service, version, rid),
            region: region,
            revisionId: rid,
            service: service,
            version: version,
        }
    }
}

impl Default for AuditDeployment {
    fn default() -> Self {
        AuditDeployment::new(&Option::None)
    }
}

pub fn audit(us: &UpgradeState, ud: &UpgradeData, audcfg: &AuditWebhook) -> Result<()> {
    let endpoint = &audcfg.url;
    debug!("state: {}, url: {:?}", us, endpoint);

    let mkerr = || ErrorKind::Url(endpoint.clone());
    let client = reqwest::Client::new();

    let ad = AuditDeployment::new(&Option::Some(ud.clone()));
    let mut ap = AuditDeploymentPayload::new("deployment", &us, &Option::Some("uidTODO".into()));
    ap.deployment = ad;

    client.post(endpoint.clone())
        .bearer_auth(audcfg.token.clone())
        .json(&ap)
        .send()
        .chain_err(&mkerr)?;
    Ok(())
}
// TODO monday: make sure that damn obj spec is working fine :D tehee

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Url;

    #[test]
    fn does_audit() {
        let audcfg = AuditWebhook{
            url: Url::parse("http://localhost:6666/events").unwrap(),
            token: "1234".into(),
        };
        let us = UpgradeState::Completed;
        let ud = UpgradeData{
            name: "svc".into(),
            chart: "wtv".into(),
            version: "v1".into(),
            region: "r1".into(),
            ..Default::default()
        };
        assert!(audit(&us, &ud, &audcfg).is_ok());
    }
}