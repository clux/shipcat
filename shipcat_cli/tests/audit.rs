#![warn(rust_2018_idioms)]

mod common;
use crate::common::setup;

use std::collections::BTreeMap;

use url::Url;

use mockito;
use shipcat;

use crate::mockito::mock;
// use mockito::Matcher;

use crate::shipcat::{audit, webhooks, AuditWebhook};
use crate::shipcat::apply::UpgradeInfo;
use shipcat_definitions::{ConfigType, Config};

#[test]
fn audit_does_audit_deployment() {
    setup();
    let mut whc: BTreeMap<String, String> = BTreeMap::default();
    whc.insert("SHIPCAT_AUDIT_CONTEXT_ID".into(), "egcontextid".into());
    whc.insert("SHIPCAT_AUDIT_CONTEXT_LINK".into(), "http://eg.server/".into());
    whc.insert("SHIPCAT_AUDIT_REVISION".into(), "egrevision".into());

    let audcfg = AuditWebhook{
        url: Url::parse(&format!("{}/audit", mockito::SERVER_URL)).unwrap(),
        token: "1234auth".into(),
    };

    let (conf, reg) = Config::new(ConfigType::Base, "dev-uk").unwrap();
    let mf = shipcat_filebacked::load_manifest("fake-ask", &conf, &reg)
        .unwrap().version("1.0.0".into());

    let us = webhooks::UpgradeState::Completed;
    let ud = UpgradeInfo::new(&mf);

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

    assert!(audit::audit_apply(&us, &ud, &audcfg, whc).is_ok());
    mocked.assert();
}

#[test]
fn audit_reconciliation_has_type() {
    let mut whc: BTreeMap<String, String> = BTreeMap::default();
    whc.insert("SHIPCAT_AUDIT_CONTEXT_ID".into(), "egcontextid".into());
    whc.insert("SHIPCAT_AUDIT_CONTEXT_LINK".into(), "http://eg.server/".into());
    whc.insert("SHIPCAT_AUDIT_REVISION".into(), "egrevision".into());

    let arp = audit::AuditReconciliationPayload::new(&whc, "region_name");
    let ae = audit::AuditEvent::new(&whc, &webhooks::UpgradeState::Completed, arp);
    assert_eq!(ae.domain_type, "reconciliation");
}
