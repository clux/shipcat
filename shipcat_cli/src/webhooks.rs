use crate::{
    audit,
    grafana,
    slack,
    Result
};
use crate::helm::{UpgradeData, UpgradeMode};
use super::{Config, Region, Webhook};

/// The different states an upgrade can be in
#[derive(Serialize, PartialEq, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UpgradeState {
    /// Before action
    Pending,
    /// No errors
    Completed,
    /// Errors
    Failed,
    // Before revert
    RollingBack,
    // After revert
    RolledBack,
    // Fail to revert
    RollbackFailed,
}

pub fn ensure_requirements(reg: &Region) -> Result<()> {
    if let Some(whs) = &reg.webhooks {
        for wh in whs {
            wh.get_configuration()?;
        }
    }
    Ok(())
}

/// Throw events to configured webhooks - warning on delivery errors
///
/// Http errors are NOT propagated from here
pub fn reconcile_event(us: UpgradeState, reg: &Region) {
    if let Some(whs) = &reg.webhooks {
        for wh in whs {
            if let Ok(whc) = wh.get_configuration() {
                if let Err(e) = match wh {
                    Webhook::Audit(h) => {
                        audit::audit_reconciliation(&us, &reg.name, &h, whc)
                    }
                } {
                    warn!("Failed to notify about reconciliation event: {}", e)
                }
            }
        }
    }
}

/// Throw events to configured webhooks - warning on delivery errors
///
/// Http errors are NOT propagated from here
pub fn upgrade_event(us: UpgradeState, ud: &UpgradeData, reg: &Region, conf: &Config) {
    //for wh in &reg.webhooks {
    //    if let Err(e) = match wh {
    //        Webhook::Audit(h) => {
    //            audit::audit_deployment(us, ud, h)
    //        }
    //    } {
    //        warn!("Failed to notify about deployment event: {}", e)
    //    }
    //}
    handle_upgrade_notifies(us, ud, &reg, &conf);
    // TODO: make a smarter loop over webhooks in here
    // TODO: first add grafana and slack to webhooks for region
}

/// Notify slack / audit endpoint of upgrades from a single upgrade
fn handle_upgrade_notifies(us: UpgradeState, ud: &UpgradeData, reg: &Region, conf: &Config) {
    if let Some(whs) = &reg.webhooks {
        for wh in whs {
            if let Ok(whc) = wh.get_configuration() {
                if let Err(e) = match wh {
                    Webhook::Audit(h) => {
                        audit::audit_deployment(&us, &ud, &h, whc)
                    }
                } {
                    warn!("Failed to notify about deployment event: {}", e)
                }
            }
        }
    }

    // Slack and Grafana

    let code = if ud.diff.is_empty() { None } else { Some(ud.diff.clone()) };
    let (color, text) = match us {
        UpgradeState::Completed => ("good".into(), format!("{} `{}` in `{}`", ud.mode.action_verb(), ud.name, ud.region)),
        UpgradeState::Failed => ("danger".into(), format!("failed to {} `{}` in `{}`", ud.mode, ud.name, ud.region)),
        _ => ("good", format!("action state: {}", serde_json::to_string(&us).unwrap_or("unknown".into()))),
    };

    match us {
        UpgradeState::Completed | UpgradeState::Failed => {
            if ud.mode != UpgradeMode::DiffOnly {
              let _ = grafana::create(grafana::Annotation {
                  event: grafana::Event::Upgrade,
                  service: ud.name.clone(),
                  version: ud.version.clone(),
                  region: ud.region.clone(),
                  time: grafana::TimeSpec::Now,
              });
            }
            let _ = slack::send(slack::Message {
                text, code,
                color: Some(String::from(color)),
                version: Some(ud.version.clone()),
                metadata: ud.metadata.clone(),
                ..Default::default()
            }, &conf, &reg.environment);
        }
        _ => {},
    }
}

/// Throw events to configured webhooks - warning on delivery errors
///
/// Http errors are NOT propagated from here
pub fn upgrade_rollback_event(us: UpgradeState, ud: &UpgradeData, reg: &Region, conf: &Config) {
    if let Some(whs) = &reg.webhooks {
        for wh in whs {
            if let Ok(whc) = wh.get_configuration() {
                if let Err(e) = match wh {
                    Webhook::Audit(h) => {
                        audit::audit_deployment(&us, &ud, &h, whc)
                    }
                } {
                    warn!("Failed to notify about rollback event: {}", e)
                }
            }
        }
    }

    if let Err(e) = match us {
        // UpgradeState::RollingBack => {},
        UpgradeState::Completed | UpgradeState::RolledBack => {
            let _ = slack::send(slack::Message {
                text: format!("rolling back `{}` in {}", &ud.name, &ud.region),
                color: Some("warning".into()),
                metadata: ud.metadata.clone(),
                ..Default::default()
            }, &conf, &reg.environment);
            grafana::create(grafana::Annotation {
                event: grafana::Event::Rollback,
                service: ud.name.clone(),
                version: ud.version.clone(),
                region: ud.region.clone(),
                time: grafana::TimeSpec::Now,
            })
        },
        UpgradeState::Failed | UpgradeState::RollbackFailed => {
            slack::send(slack::Message {
                text: format!("failed to rollback `{}` in {}", &ud.name, &ud.region),
                color: Some("danger".into()),
                metadata: ud.metadata.clone(),
                ..Default::default()
            }, &conf, &reg.environment)
        },
        _ => { Ok(()) },
    } {
        warn!("Failed to notify about rollback event: {}", e);
    }
    // TODO: make a smarter loop over webhooks in here
    // TODO: first add grafana and slack to webhooks for region
}
