use std::collections::BTreeMap;

use serde::Serialize;

use url::Url;
use chrono::{Utc, SecondsFormat};

use crate::webhooks::UpgradeState;
use super::{Result, ResultExt, ErrorKind};
use super::{AuditWebhook};
use crate::helm::direct::UpgradeData;

/// Payload that gets sent via audit webhook
#[derive(Serialize, Clone)]
pub struct AuditEvent<T>
where T: Serialize + Clone + AuditType {
    /// Payload type
    #[serde(rename = "type")]
    pub domain_type: String,
    /// RFC 3339
    pub timestamp: String,
    pub status: UpgradeState,
    /// Eg a jenkins job id
    pub context_id: String,
    /// Eg a jenkins job url
    #[serde(with = "url_serde", skip_serializing_if = "Option::is_none")]
    pub context_link: Option<Url>,

    /// represents a single helm upgrade or a reconciliation
    pub payload: T,
}

impl<T> AuditEvent<T>
where T: Serialize + Clone + AuditType {
    /// Timestamped payload skeleton
    pub fn new(whc: &BTreeMap<String, String>, status: &UpgradeState, payload: T) -> Self {
        AuditEvent{
            domain_type: AuditType::get_domain_type(&payload),
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            status: status.clone(),
            context_id: whc["SHIPCAT_AUDIT_CONTEXT_ID"].clone(),
            context_link: whc.get("SHIPCAT_AUDIT_CONTEXT_LINK")
                            .and_then(|l| Url::parse(&l).ok()),
            payload
        }
    }
}

pub trait AuditType {
    fn get_domain_type(&self) -> String;
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
    pub fn new(whc: &BTreeMap<String, String>, ud: &UpgradeData) -> Self {
        let (service, region, version) = (ud.name.clone(), ud.region.clone(), ud.version.clone());
        let manifests_revision = whc["SHIPCAT_AUDIT_REVISION"].clone();
        Self {
            id: format!("{}-{}-{}-{}", manifests_revision, region, service, version),
            manifests_revision, region, service, version,
        }
    }
}

impl AuditType for AuditDeploymentPayload {
    fn get_domain_type(&self) -> String {
        "deployment".into()
    }
}

impl AuditReconciliationPayload {
    pub fn new(whc: &BTreeMap<String, String>, r: &str) -> Self {
        let manifests_revision = whc["SHIPCAT_AUDIT_REVISION"].clone();
        let region = r.into();
        Self {
            id: format!("{}-{}", manifests_revision, region),
            manifests_revision, region,
        }
    }
}

impl AuditType for AuditReconciliationPayload {
    fn get_domain_type(&self) -> String {
        "reconciliation".into()
    }
}

pub fn audit_deployment(us: &UpgradeState, ud: &UpgradeData, audcfg: &AuditWebhook, whc: BTreeMap<String, String>) -> Result<()> {
    let ae = AuditEvent::new(&whc, &us, AuditDeploymentPayload::new(&whc, &ud));
    audit(ae, &audcfg)
}

pub fn audit_reconciliation(us: &UpgradeState, region: &str, audcfg: &AuditWebhook, whc: BTreeMap<String, String>) -> Result<()> {
    let ae = AuditEvent::new(&whc, &us, AuditReconciliationPayload::new(&whc, region));
    audit(ae, &audcfg)
}

fn audit<T: Serialize + Clone + AuditType>(ae: AuditEvent<T>, audcfg: &AuditWebhook) -> Result<()> {
    let endpoint = &audcfg.url;
    debug!("event status: {}, url: {:?}", serde_json::to_string(&ae.status)?, endpoint);

    let mkerr = || ErrorKind::Url(endpoint.clone());
    let client = reqwest::Client::new();

    let _res = client.post(endpoint.clone())
        .bearer_auth(audcfg.token.clone())
        .json(&ae)
        .send()
        .chain_err(&mkerr)?;
    // TODO: check _res.is_success if it's a requirement in future
    Ok(())
}
