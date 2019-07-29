use crate::{
    audit,
    grafana,
    slack,
    Result
};
use crate::apply::UpgradeInfo;
use crate::helm::{UpgradeData, UpgradeMode};
use super::{Config, Region, Webhook};

/// The different states an upgrade can be in
#[derive(Serialize, PartialEq, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UpgradeState {
    /// Before action
    Pending,
    // Action has started
    Started,
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
        wh.get_configuration()?;
    }
    Ok(())
}

/// Throw events to configured webhooks - warning on delivery errors
///
/// Http errors are NOT propagated from here
pub fn reconcile_event(us: UpgradeState, reg: &Region) {
    for wh in &reg.webhooks {
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

/// Throw events to configured webhooks - warning on delivery errors
///
/// Http errors are NOT propagated from here
/// TODO: remove once apply_event used everywhere
pub fn upgrade_event(us: UpgradeState, ud: &UpgradeData, reg: &Region, conf: &Config) {
    handle_upgrade_notifies(us, ud, &reg, &conf);
    // TODO: make a smarter loop over webhooks in here
    // TODO: first add grafana and slack to webhooks for region
}


/// Throw events to configured webhooks
///
/// This is the new version for shipcat apply module
pub fn apply_event(us: UpgradeState, info: &UpgradeInfo, reg: &Region, conf: &Config) {
    // Webhooks defined in shipcat.conf for the region:
    for wh in &reg.webhooks {
        if let Ok(whc) = wh.get_configuration() {
            let res = match wh {
                Webhook::Audit(h) => audit::audit_apply(&us, &info, &h, whc)
            };
            if let Err(e) = res {
                warn!("Failed to notify about apply event: {}", e)
            }
        }
    }

    // slack notifications:
    let (color, text) = match us {
        UpgradeState::Completed => (
                "good".into(),
                format!("applied `{}` in `{}`", info.name, info.region)
            ),
        UpgradeState::Failed => (
                "danger".into(),
                format!("failed to apply `{}` in `{}`", info.name, info.region)
            ),
        _ => (
                "good",
                format!("action state: {}", serde_json::to_string(&us).unwrap_or("unknown".into()))
            ),
    };
    match us {
        UpgradeState::Completed | UpgradeState::Failed => {
            let _ = slack::send(slack::Message {
                text,
                code: info.diff.clone(),
                color: Some(String::from(color)),
                version: Some(info.version.clone()),
                metadata: info.metadata.clone(),
            }, &conf, &reg.environment);
        }
        _ => {},
    }

    // grafana annotations:
    match us {
        UpgradeState::Completed | UpgradeState::Failed => {
            let _ = grafana::create(grafana::Annotation {
                event: grafana::Event::Upgrade,
                service: info.name.clone(),
                version: info.version.clone(),
                region: info.region.clone(),
                time: grafana::TimeSpec::Now,
            });
        }
        _ => {},
    }
}


/// Notify slack / audit endpoint of upgrades from a single upgrade
///
/// TODO: remove once helm module disappears
fn handle_upgrade_notifies(us: UpgradeState, ud: &UpgradeData, reg: &Region, conf: &Config) {
    for wh in &reg.webhooks {
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
                metadata: ud.metadata.clone().unwrap(),
            }, &conf, &reg.environment);
        }
        _ => {},
    }
}

/// Throw events to configured webhooks - warning on delivery errors
///
/// Http errors are NOT propagated from here
/// TODO: remove once we kill this functionality
pub fn upgrade_rollback_event(us: UpgradeState, ud: &UpgradeData, reg: &Region, conf: &Config) {
    for wh in &reg.webhooks {
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

    if let Err(e) = match us {
        // UpgradeState::RollingBack => {},
        UpgradeState::Completed | UpgradeState::RolledBack => {
            let _ = slack::send(slack::Message {
                text: format!("rolling back `{}` in {}", &ud.name, &ud.region),
                color: Some("warning".into()),
                metadata: ud.metadata.clone().unwrap(),
                code: None, version: None,
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
                metadata: ud.metadata.clone().unwrap(),
                code: None, version: None,
            }, &conf, &reg.environment)
        },
        _ => { Ok(()) },
    } {
        warn!("Failed to notify about rollback event: {}", e);
    }
    // TODO: make a smarter loop over webhooks in here
    // TODO: first add grafana and slack to webhooks for region
}
