use serde::Serialize;
use std::env;

use url::Url;
use chrono::{Utc, SecondsFormat};

use super::{Result, ResultExt, ErrorKind};
use super::{Webhooks, AuditWebhook};
use crate::helm::direct::{UpgradeData, UpgradeState};

/// Payload that gets sent via audit webhook
#[derive(Serialize, Clone)]
pub struct AuditEvent<T> where T: Serialize + Clone {
    /// RFC 3339
    pub timestamp: String,
    pub status: UpgradeState,
    /// Eg a jenkins job id
    pub context_id: Option<String>,
    /// Eg a jenkins job url
    #[serde(with = "url_serde", skip_serializing_if = "Option::is_none")]
    pub context_link: Option<Url>,

    /// represents a single helm upgrade or a reconciliation
    pub payload: T,
}

impl<T> AuditEvent<T> where T: Serialize + Clone {
    /// Timestamped payload skeleton
    pub fn new(us: &UpgradeState, payload: T) -> Self {
        AuditEvent{
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            status: us.clone(),
            context_id: env::var("SHIPCAT_AUDIT_CONTEXT_ID").ok(),
            context_link: if let Ok(l) = env::var("SHIPCAT_AUDIT_CONTEXT_LINK") {
                Url::parse(&l).ok()
            } else { None },
            payload
        }
    }
}

#[derive(Serialize, Clone)]
pub struct AuditDeploymentPayload {
    id: String,
    region: String,
    /// Eg Git SHA
    manifests_revision: String,
    service: String,
    version: String,
}

#[derive(Serialize, Clone)]
pub struct AuditReconciliationPayload {
    id: String,
    region: String,
    /// Eg Git SHA
    manifests_revision: String,
}

impl AuditDeploymentPayload {
    pub fn new(udopt: Option<UpgradeData>) -> Self {
        let (service, region, version) = if let Some(ud) = udopt {
            (ud.name.clone(), ud.region.clone(), ud.version.clone())
        } else {
            ("unknown".into(), "unknown".into(), "unknown".into())
        };
        let manifests_revision = env::var("SHIPCAT_AUDIT_REVISION").unwrap_or("undefined".into());

        Self {
            id: format!("{}-{}-{}-{}", manifests_revision, region, service, version),
            manifests_revision, region, service, version,
        }
    }
}
impl AuditReconciliationPayload {
    pub fn new(r: &str) -> Self {
        let manifests_revision = env::var("SHIPCAT_AUDIT_REVISION").unwrap_or("undefined".into());

        let region = r.into();
        Self {
            id: format!("{}-{}", manifests_revision, region),
            manifests_revision, region,
        }
    }
}

pub fn ensure_requirements(wh: Option<Webhooks>) -> Result<()> {
    if let Some(_) = &wh {
        // Assume that webhooks strictly contains audit struct if present
        env::var("SHIPCAT_AUDIT_CONTEXT_ID").map_err(|_| ErrorKind::MissingAuditContextId.to_string())?;
        env::var("SHIPCAT_AUDIT_REVISION").map_err(|_| ErrorKind::MissingAuditRevision.to_string())?;
    }
    Ok(())
}

pub fn audit_deployment(us: &UpgradeState, ud: &UpgradeData, audcfg: &AuditWebhook) -> Result<()> {
    let ae = AuditEvent::new(&us, AuditDeploymentPayload::new(Some(ud.clone())));
    audit(&ae, &audcfg)
}

pub fn audit_reconciliation(us: &UpgradeState, region: &str, audcfg: &AuditWebhook) -> Result<()> {
    let ae = AuditEvent::new(&us, AuditReconciliationPayload::new(region));
    audit(&ae, &audcfg)
}

fn audit<T: Serialize + Clone>(ae: &AuditEvent<T>, audcfg: &AuditWebhook) -> Result<()> {
    let endpoint = &audcfg.url;
    debug!("event status: {}, url: {:?}", ae.status, endpoint);

    let mkerr = || ErrorKind::Url(endpoint.clone());
    let client = reqwest::Client::new();

    let _res = client.post(endpoint.clone())
        .bearer_auth(audcfg.token.clone())
        .json(&ae)
        .send()
        .chain_err(&mkerr)?;
    // TODO: check _res.is_success
    Ok(())
}
