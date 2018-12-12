#![warn(rust_2018_idioms)]

mod common;

use std::env;
use url::Url;

use mockito;
use shipcat;

use crate::mockito::mock;
// use mockito::Matcher;

use crate::shipcat::audit;
use crate::shipcat::{AuditWebhook};
use crate::shipcat::helm::direct::UpgradeData;
use crate::shipcat::webhooks;

#[test]
fn audit_does_audit_deployment() {
    env::set_var("SHIPCAT_AUDIT_CONTEXT_ID", "egcontextid");
    env::set_var("SHIPCAT_AUDIT_CONTEXT_LINK", "http://eg.server/");
    env::set_var("SHIPCAT_AUDIT_REVISION", "egrevision");

    let audcfg = AuditWebhook{
        url: Url::parse(&format!("{}/audit", mockito::SERVER_URL)).unwrap(),
        token: "1234auth".into(),
    };
    let us = webhooks::UpgradeState::Completed;
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

    assert!(audit::audit_deployment(&us, &ud, &audcfg).is_ok());
    mocked.assert();
}

#[test]
fn audit_reconciliation_has_type() {
    let arp = audit::AuditReconciliationPayload::new("region_name");
    let ae = audit::AuditEvent::new(&webhooks::UpgradeState::Completed, arp);
    assert_eq!(ae.domain_type, "reconciliation");
}
