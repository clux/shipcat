use std::env;

use url::Url;
use chrono::{Utc, SecondsFormat};

use super::{Result, ResultExt, ErrorKind};
use super::AuditWebhook;
use helm::direct::{UpgradeData, UpgradeState};

/// Payload that gets sent via audit webhook
#[derive(Serialize, Clone, Default)]
#[cfg_attr(test, derive(Debug))]
#[serde(rename_all = "snake_case")]
struct AuditDeploymentPayload {
    #[serde(rename = "type")]
    pub domainType: String,
    /// RFC 3339
    pub timestamp: String,
    pub status: UpgradeState,
    /// Eg a jenkins job id
    pub contextId: Option<String>,
    /// Eg a jenkins job url
    #[serde(with = "url_serde")]
    pub contextLink: Option<Url>,
    /// Domain Object
    pub deployment: AuditDeployment,
}

impl AuditDeploymentPayload {
    /// Timestamped payload skeleton
    pub fn new(dt: &str, us: &UpgradeState) -> Self {
        AuditDeploymentPayload{
            domainType: String::from(dt),
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            status: us.clone(),
            contextId: env::var("SHIPCAT_AUDIT_CONTEXT_ID").ok(),
            contextLink: if let Some(urlstr) = env::var("SHIPCAT_AUDIT_CONTEXT_LINK").ok() {
                url::Url::parse(&urlstr).ok()
            } else { None },
            ..Default::default()
        }
    }
}

/// Payload for single deployment domain object
#[derive(Serialize, Clone)]
#[cfg_attr(test, derive(Debug))]
#[serde(rename_all = "snake_case")]
struct AuditDeployment {
    pub id: String,
    pub region: String,
    /// Eg Git SHA
    pub manifestsRevision: String,
    pub service: String,
    pub version: String,
}

impl AuditDeployment {
    pub fn new(udopt: Option<UpgradeData>, ) -> Self {
        let (service, region, version) = if let Some(ud) = udopt {
            (ud.name.clone(), ud.region.clone(), ud.version.clone())
        } else {
            ("unknown".into(), "unknown".into(), "unknown".into())
        };
        let manifestsRevision = match env::var("SHIPCAT_AUDIT_REVISION") {
            Ok(ev) => ev,
            Err(e) => panic!(e),
        };

        AuditDeployment{
            id: format!("{}-{}-{}", service, version, manifestsRevision),
            region, manifestsRevision, service, version,
        }
    }
}

impl Default for AuditDeployment {
    fn default() -> Self {
        AuditDeployment::new(None)
    }
}

pub fn audit(us: &UpgradeState, ud: &UpgradeData, audcfg: &AuditWebhook) -> Result<()> {
    let endpoint = &audcfg.url;
    debug!("state: {}, url: {:?}", us, endpoint);

    let mkerr = || ErrorKind::Url(endpoint.clone());
    let client = reqwest::Client::new();

    let ad = AuditDeployment::new(Some(ud.clone()));
    let mut ap = AuditDeploymentPayload::new("deployment", &us);
    ap.deployment = ad;

    client.post(endpoint.clone())
        .bearer_auth(audcfg.token.clone())
        .json(&ap)
        .send()
        .chain_err(&mkerr)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    extern crate mockito;

    use std::env;
    use super::*;
    use reqwest::Url;
    use self::mockito::mock;
    // use self::mockito::Matcher;

    #[test]
    fn does_audit() {
        env::set_var("SHIPCAT_AUDIT_CONTEXT_ID", "egcontextid");
        env::set_var("SHIPCAT_AUDIT_CONTEXT_LINK", "http://eg.server/");
        env::set_var("SHIPCAT_AUDIT_REVISION", "egrevision");

        let audcfg = AuditWebhook{
            url: Url::parse(&format!("{}/audit", mockito::SERVER_URL)).unwrap(),
            token: "1234auth".into(),
        };
        let us = UpgradeState::Completed;
        let ud = UpgradeData{
            name: "svc".into(),
            chart: "wtv".into(),
            version: "v1".into(),
            region: "r1".into(),
            ..Default::default()
        };

        let mocked = mock("POST", "/audit")
            .match_header("content-type", "application/json")
            .match_header("Authorization", "Bearer 1234auth")
            // TODO: match body with frozen timestamp
            // .match_body(Matcher::Json(json!(
            //     {
            //         "type": "deployment",
            //         "timestamp": "frozen",
            //         "status": "COMPLETED",
            //         "contextId": "egcontextid",
            //         "contextLink": "http://eg.server/",
            //         "deployment": {
            //             "id": "svc-v1-egrevision",
            //             "region": "r1",
            //             "manifestsRevision": "egrevision",
            //             "service": "svc",
            //             "version": "v1"
            //         }
            //     }
            // )))
            .expect(1)
            .create();

        assert!(audit(&us, &ud, &audcfg).is_ok());
        mocked.assert();

    }
}
