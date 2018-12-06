use std::env;

use url::Url;
use chrono::{Utc, SecondsFormat};

use super::{Result, ResultExt, ErrorKind};
use super::AuditWebhook;
use helm::direct::{UpgradeData, UpgradeState};

/// Payload that gets sent via audit webhook
#[derive(Serialize, Clone)]
#[cfg_attr(test, derive(Debug))]
// #[serde(rename_all = "snake_case")] // well, it just didn't work. :/ maybe on a later serde version?
struct AuditEvent {
    /// RFC 3339
    pub timestamp: String,
    pub status: UpgradeState,
    /// Eg a jenkins job id
    #[serde(rename = "context_id")]
    pub contextId: Option<String>,
    /// Eg a jenkins job url
    #[serde(rename = "context_link", with = "url_serde", skip_serializing_if = "Option::is_none")]
    pub contextLink: Option<Url>,

    /// represents a single helm upgrade or a reconciliation
    #[serde(flatten)]
    pub payload: AuditDomainObject,
}

impl AuditEvent {
    /// Timestamped payload skeleton
    pub fn new(us: &UpgradeState) -> Self {
        AuditEvent{
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            status: us.clone(),
            contextId: env::var("SHIPCAT_AUDIT_CONTEXT_ID").ok(),
            contextLink: if let Some(urlstr) = env::var("SHIPCAT_AUDIT_CONTEXT_LINK").ok() {
                url::Url::parse(&urlstr).ok()
            } else { None },
            payload: AuditDomainObject::Empty,
        }
    }
}

#[derive(Serialize, Clone)]
#[cfg_attr(test, derive(Debug))]
#[serde(tag = "type", content = "payload", rename_all="snake_case")]
enum AuditDomainObject {
    Deployment {
        id: String,
        region: String,
        /// Eg Git SHA
        #[serde(rename = "manifests_revision")]
        manifestsRevision: String,
        service: String,
        version: String,
    },
    Reconciliation {
        id: String,
        region: String,
        /// Eg Git SHA
        #[serde(rename = "manifests_revision")]
        manifestsRevision: String,
    },
    Empty,
}

impl AuditDomainObject {
    pub fn new_deployment(udopt: Option<UpgradeData>) -> Self {
        let (service, region, version) = if let Some(ud) = udopt {
            (ud.name.clone(), ud.region.clone(), ud.version.clone())
        } else {
            ("unknown".into(), "unknown".into(), "unknown".into())
        };
        let manifestsRevision = match env::var("SHIPCAT_AUDIT_REVISION") {
            Ok(ev) => ev,
            Err(e) => panic!(e),
        };

        AuditDomainObject::Deployment{
            id: format!("{}-{}-{}-{}", manifestsRevision, region, service, version),
            manifestsRevision, region, service, version,
        }
    }

    pub fn new_reconciliation(udopt: Option<UpgradeData>) -> Self {
        let region = if let Some(ud) = udopt {
            ud.region.clone()
        } else {
            "unknown".into()
        };
        let manifestsRevision = match env::var("SHIPCAT_AUDIT_REVISION") {
            Ok(ev) => ev,
            Err(e) => panic!(e),
        };

        AuditDomainObject::Reconciliation{
            id: format!("{}-{}", manifestsRevision, region),
            manifestsRevision, region,
        }
    }
}

pub fn audit(us: &UpgradeState, ud: &UpgradeData, audcfg: &AuditWebhook) -> Result<()> {
    let endpoint = &audcfg.url;
    debug!("state: {}, url: {:?}", us, endpoint);

    let mkerr = || ErrorKind::Url(endpoint.clone());
    let client = reqwest::Client::new();
    let mut ap = AuditEvent::new(&us);
    ap.payload = AuditDomainObject::new_deployment(Some(ud.clone()));

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
            //         "timestamp": "frozen",
            //         "status": "COMPLETED",
            //         "context_id": "egcontextid",
            //         "context_link": "http://eg.server/",
            //         "type": "deployment",
            //         "payload": {
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
