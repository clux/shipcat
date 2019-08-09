use crate::{
    audit,
    grafana,
    slack,
    Result
};
use crate::apply::UpgradeInfo;
use super::{Config, Region, Webhook};

/// The different states an upgrade can be in
#[derive(Serialize, PartialEq, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UpgradeState {
    /// Before action
    Pending,
    /// Action was cancelled before start
    Cancelled,
    // Action has started
    Started,
    /// No errors
    Completed,
    /// Errors
    Failed,
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
            let res = match wh {
                Webhook::Audit(h) => audit::audit_reconciliation(&us, &reg.name, &h, whc)
            };
            if let Err(e) = res {
                warn!("Failed to notify about reconciliation event: {}", e)
            }
        }
    }
}


/// Throw events to configured webhooks
///
/// This is the new version for shipcat apply module
pub fn apply_event(us: UpgradeState, info: &UpgradeInfo, reg: &Region, conf: &Config) {
    // Webhooks defined in shipcat.conf for the region:
    for wh in &reg.webhooks {
        if let Ok(whc) = wh.get_configuration() {
            let res = match wh {
                Webhook::Audit(h) => {
                    match us {
                        UpgradeState::Started |
                        UpgradeState::Completed |
                        UpgradeState::Failed => audit::audit_apply(&us, &info, &h, whc),
                        _ => Ok(()), // audit only sends Started / Failed / Completed
                    }
                }
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
