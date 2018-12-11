use std::env;
use crate::{
    audit,
    grafana,
    slack,
    Result,
    ErrorKind,
};
use crate::helm::{UpgradeData, UpgradeMode};
use super::{Region, Webhook};

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
    for wh in &reg.webhooks {
        match wh {
            Webhook::Audit(_) => {
                env::var("SHIPCAT_AUDIT_CONTEXT_ID").map_err(|_| ErrorKind::MissingAuditContextId.to_string())?;
                env::var("SHIPCAT_AUDIT_REVISION").map_err(|_| ErrorKind::MissingAuditRevision.to_string())?;
            }
        }
    }
    Ok(())
    // TODO: when slack webhook is cfged, require this:
    // slack::have_credentials()?;
}

/// Throw events to configured webhooks - warning on delivery errors
///
/// Http errors are NOT propagated from here
pub fn reconcile_event(state: UpgradeState, reg: &Region) {
    for wh in &reg.webhooks {
        if let Err(e) = match wh {
            Webhook::Audit(h) => {
                audit::audit_reconciliation(&state, &reg.name, &h)
            }
        } {
            warn!("Failed to notify about reconciliation event: {}", e)
        }
    }
}

/// Throw events to configured webhooks - warning on delivery errors
///
/// Http errors are NOT propagated from here
pub fn upgrade_event(state: UpgradeState, ud: &UpgradeData, reg: &Region) {
    //for wh in &reg.webhooks {
    //    if let Err(e) = match wh {
    //        Webhook::Audit(h) => {
    //            audit::audit_deployment(state, ud, h)
    //        }
    //    } {
    //        warn!("Failed to notify about deployment event: {}", e)
    //    }
    //}
    handle_upgrade_notifies(state, ud, &reg);
    // TODO: make a smarter loop over webhooks in here
    // TODO: first add grafana and slack to webhooks for region
}


/// Notify slack / audit endpoint of upgrades from a single upgrade
pub fn handle_upgrade_notifies(us: UpgradeState, ud: &UpgradeData, r: &Region) {
    for wh in &r.webhooks {
       if let Err(e) = match wh {
           Webhook::Audit(h) => {
               audit::audit_deployment(&us, ud, &h)
           }
       } {
           warn!("Failed to notify about deployment event: {}", e)
       }
    }
    
    // Slack and Grafana

    let code = if ud.diff.is_empty() { None } else { Some(ud.diff.clone()) };
    let (color, text) = match us {
        UpgradeState::Completed => {
            info!("successfully rolled out {}", ud.name);
            ("good".into(), format!("{} `{}` in `{}`", ud.mode.action_verb(), ud.name, ud.region))
        }
        UpgradeState::Failed => {
            warn!("failed to roll out {}", ud.name);
            ("danger".into(), format!("failed to {} `{}` in `{}`", ud.mode, ud.name, ud.region))
        }
        _ => ("good", format!("action state: {}", serde_json::to_string(&us).unwrap_or("unknown".into())))
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
            });
        }
        _ => {},
    }
}

/// Throw events to configured webhooks - warning on delivery errors
///
/// Http errors are NOT propagated from here
pub fn upgrade_rollback_event(us: UpgradeState, ud: &UpgradeData, r: &Region) {
    for wh in &r.webhooks {
        if let Err(e) = match wh {
            Webhook::Audit(h) => {
                audit::audit_deployment(&us, &ud, &h)
            }
        } {
            warn!("Failed to notify about rollback event: {}", e)
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
            });
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
            })
        },
        _ => { Ok(()) },
    } {
        warn!("Failed to notify about rollback event: {}", e);
    }
    // TODO: make a smarter loop over webhooks in here
    // TODO: first add grafana and slack to webhooks for region
}
