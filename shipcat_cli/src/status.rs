use crate::{Result, Error, ErrorKind, Manifest};
use serde::Serialize;
use serde_json::json;
use chrono::{Utc, SecondsFormat};

use kube::{
    api::{Api, Object, PatchParams},
    client::APIClient,
};

fn make_date() -> String {
    // Format == `1996-12-19T16:39:57-08:00`, but we hardcode Utc herein.
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Status object for shipcatmanifests crd
///
/// All fields optional, but we try to ensure all fields exist.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ManifestStatus {
    /// Detailed individual conditions, emitted as they happen during apply
    #[serde(default)]
    pub conditions: Conditions,
    /// A more easily readable summary of why the conditions are what they are
    #[serde(default)]
    pub summary: Option<ConditionSummary>,

    // TODO: vault secret hash
    // MAYBE: kong status?
    // MAYBE: canary status?
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Conditions {
    /// Generated
    ///
    /// If this .status is false, this might contain information about:
    /// - manifest failing to complete
    /// - temporary manifest files failing to write to disk
    /// - manifests failing to serialize
    /// - secrets failing to resolve
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated: Option<Condition>,

    /// Applied status
    ///
    /// If applied.status is false, this might contain information about:
    /// - invalid yaml when combining charts and values
    /// - configuration not passing admission controllers logic
    /// - network errors when applying
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied: Option<Condition>,

    /// Rollout of current shipcatmanifest succeeded
    ///
    /// If rollout.status is false, this might contain information about:
    /// - deployment(s) failing to roll out in time
    /// - network errors tracking the rollout
    /// Best effort information given in message, but this won't replace DeploymentConditions
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rolledout: Option<Condition>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConditionSummary {
    /// Date string (RFC3339) of when we generated the template successfully
    #[serde(default)]
    last_successful_generate: Option<String>,

    /// Date string (RFC3339) of when we last applied manifest configuration
    #[serde(default)]
    pub last_apply: Option<String>,

    /// Date string (RFC3339) of when an apply passed all checks
    #[serde(default)]
    last_successful_apply: Option<String>,

    /// Date string (RFC3339) of when a rollout wait completed
    #[serde(default)]
    last_rollout: Option<String>,

    /// Date string (RFC3339) of when a rollout wait completed and passed
    #[serde(default)]
    last_successful_rollout: Option<String>,

    // last action we performed
    #[serde(default)]
    last_action: Option<String>,

    /// reason for last failure (if any)
    #[serde(default)]
    last_failure_reason: Option<String>,

    /// Best effort reason for why an apply was triggered
    #[serde(default)]
    last_apply_reason: Option<String>,
}

/// Condition
///
/// Stated out like a normal kubernetes conditions like PodCondition:
///
///  - lastProbeTime: null
///    lastTransitionTime: "2019-07-31T13:07:30Z"
///    message: 'containers with unready status: [product-config]'
///    reason: ContainersNotReady
///    status: "False"
///    type: ContainersReady
///
/// where we ignore lastProbeTime / lastHeartbeatTime because they are expensive,
/// and we add in an originator/source of the condition for parallel setups.
///
/// However, due to the lack of possibilities for patching statuses and general
/// difficulty dealing with the vector struct, we instead have multiple named variants.
///
/// See https://github.com/kubernetes/kubernetes/issues/7856#issuecomment-323196033
/// and https://github.com/clux/kube-rs/issues/43
/// For the reasoning.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Condition {
    /// Whether or not in a good state
    ///
    /// This must default to true when in a good state
    pub status: bool,
    /// Error reason type if not in a good state
    #[serde(default)]
    pub reason: Option<String>,
    /// One sentence error message if not in a good state
    #[serde(default)]
    pub message: Option<String>,

    /// When the condition was last written (RFC 3339 timestamp)
    #[serde(rename = "lastTransitionTime")]
    pub last_transition: String,

    /// Originator for this condition
    #[serde(default)]
    pub source: Option<Applier>,
}

impl Condition {
    fn ok(a: &Applier) -> Self {
        Condition {
            status: true,
            source: Some(a.clone()),
            last_transition: make_date(),
            reason: None,
            message: None,
        }
    }

    fn bad(a: &Applier, err: &str, msg: String) -> Self {
        Condition {
            status: false,
            source: Some(a.clone()),
            last_transition: make_date(),
            reason: Some(err.into()),
            message: Some(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Applier, Condition};
    use chrono::Utc;
    use chrono::prelude::*;
    #[test]
    #[ignore]
    fn check_conditions() {
        let applier = Applier { name: "clux".into(), url: None };
        let mut cond = Condition::ok(&applier);
        cond.last_transition = Utc.ymd(1996, 12, 19)
            .and_hms(16, 39, 57)
            .to_rfc3339_opts(SecondsFormat::Secs, true);
        let encoded = serde_yaml::to_string(&cond).unwrap();
        println!("{}", encoded);
        assert!(encoded.contains("status: true"));
        assert!(encoded.contains("lastTransitionTime: \"1996-12-19T16:39:57+00:00\""));
    }
}


#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct Applier {
    /// Human readable text describing what applied
    pub name: String,
    /// Link to logs or origin of the apply (if possible)
    #[serde(default)]
    pub url: Option<String>,
}

impl Applier {
    /// Infer originator of an apply
    pub fn infer() -> Applier {
        use std::env;
        if let (Ok(url), Ok(name), Ok(nr)) = (env::var("BUILD_URL"),
                                              env::var("JOB_NAME"),
                                              env::var("BUILD_NUMBER")) {
            // we are on jenkins
            Applier { name: format!("{}#{}", name, nr), url: Some(url) }
        } else if let (Ok(url), Ok(name), Ok(nr)) = (env::var("CIRCLE_BUILD_URL"),
                                                     env::var("CIRCLE_JOB"),
                                                     env::var("CIRCLE_BUILD_NUM")) {
            // we are on circle
            Applier { name: format!("{}#{}", name, nr), url: Some(url) }
        } else if let Ok(user) = env::var("USER") {
            Applier { name: user, url: None }
        } else {
            warn!("Could not infer applier from this environment");
            // TODO: maybe lock down this..
            Applier { name: "unknown origin".into(), url: None }
        }
    }
}


/// Client creator
///
/// TODO: embed inside shipcat::apply when needed for other things
fn make_client() -> Result<APIClient> {
    let config = kube::config::incluster_config().or_else(|_| {
        kube::config::load_kube_config()
    }).map_err(|_e| Error::from(ErrorKind::KubeError))?;
    Ok(kube::client::APIClient::new(config))
}

/// Kube Object version of Manifest
///
/// This is the new version of Crd<Manifest> (which will be removed)
type ManifestK = Object<Manifest, ManifestStatus>;

/// Interface for dealing with status
pub struct Status {
    /// This allow graceful degradation by wrapping it in an option
    scm: Api<ManifestK>,
    applier: Applier,
    name: String,
}

/// Entry points for shipcat::apply
impl Status {
    pub fn new(mf: &Manifest) -> Result<Self> {
        // hide the client in here -> Api resource for now (not needed elsewhere)
        let client = make_client()?;
        let scm : Api<ManifestK> = Api::customResource(client, "shipcatmanifests")
            .group("babylontech.co.uk")
            .within(&mf.namespace);
        Ok(Status {
            name: mf.name.clone(),
            applier: Applier::infer(),
            scm: scm,
        })
    }

    /// CRD applier
    pub fn apply(&self, mf: Manifest) -> Result<bool> {
        assert!(mf.version.is_some()); // ensure crd is in right state w/o secrets
        assert!(mf.is_base());
        // TODO: use server side apply in 1.15
        //let mfk = json!({
        //    "apiVersion": "babylontech.co.uk/v1",
        //    "kind": "ShipcatManifest",
        //    "metadata": {
        //        "name": mf.name,
        //        "namespace": mf.namespace,
        //    },
        //    "spec": mf,
        //});
        // for now, shell out to kubectl
        use crate::kubectl;
        let svc = mf.name.clone();
        let ns = mf.namespace.clone();
        kubectl::apply_crd(&svc, mf, &ns)
    }

    /// Full CRD fetcher
    pub fn get(&self) -> Result<ManifestK> {
        let o = self.scm.get(&self.name).map_err(|e| {
            warn!("KubeError: {}", e);
            Error::from(ErrorKind::KubeError) // TODO: FIX CHAIN
        })?;
        Ok(o)
    }

    // ====================================================
    // WARNING : PATCH HELL BELOW
    // ====================================================

    // helper to send a merge patch
    fn patch(&self, data: &serde_json::Value) -> Result<ManifestK> {
        let pp = PatchParams::default();
        let o = self.scm.patch_status(&self.name, &pp, serde_json::to_vec(data)?)
            .map_err(|e| {
                warn!("KubeError: {}", e);
                Error::from(ErrorKind::KubeError)
            })?;
            //.chain_err(|| ErrorKind::KubeError)?; can't atm because it's a failure..
        debug!("Patched status: {:?}", o.status);
        Ok(o)
    }

    // helper to delete accidental flags
    pub fn update_generate_true(&self) -> Result<ManifestK> {
        debug!("Setting generated true");
        let now = make_date();
        let cond = Condition::ok(&self.applier);
        let data = json!({
            "status": {
                "conditions": {
                    "generated": cond
                },
                "summary": {
                    "lastSuccessfulGenerate": now,
                    "lastAction": "Generate",
                }
            }
        });
        self.patch(&data)
    }

    // Manual helper fn to blat old status data
    #[allow(dead_code)]
    fn remove_old_props(&self) -> Result<ManifestK> {
        // did you accidentally populate the .status object with garbage?
        let _data = json!({
            "status": {
                "conditions": {
                    "apply": null,
                    "rollout": null,
                },
                "summary": null
            }
        });
        unreachable!("I know what i am doing");
        #[allow(unreachable_code)]
        self.patch(&_data)
    }

    pub fn update_generate_false(&self, err: &str, reason: String) -> Result<ManifestK> {
        debug!("Setting generated false");
        let cond = Condition::bad(&self.applier, err, reason.clone());
        let data = json!({
            "status": {
                "conditions": {
                    "generated": cond
                },
                "summary": {
                    "lastFailureReason": reason,
                    "lastAction": "Generate",
                }
            }
        });
        self.patch(&data)
    }

    pub fn update_apply_true(&self, ureason: String) -> Result<ManifestK> {
        debug!("Setting applied true");
        let now = make_date();
        let cond = Condition::ok(&self.applier);
        let data = json!({
            "status": {
                "conditions": {
                    "applied": cond
                },
                "summary": {
                    "lastApply": now,
                    "lastSuccessfulApply": now,
                    "lastApplyReason": ureason,
                    "lastAction": "Apply",
                }
            }
        });
        self.patch(&data)
    }

    pub fn update_apply_false(&self, ureason: String, err: &str, reason: String) -> Result<ManifestK> {
        debug!("Setting applied false");
        let now = make_date();
        let cond = Condition::bad(&self.applier, err, reason.clone());
        let data = json!({
            "status": {
                "conditions": {
                    "applied": cond
                },
                "summary": {
                    "lastApply": now,
                    "lastFailureReason": reason,
                    "lastApplyReason": ureason,
                    "lastAction": "Apply",
                }
            }
        });
        self.patch(&data)
    }

    pub fn update_rollout_false(&self, err: &str, reason: String) -> Result<ManifestK> {
        debug!("Setting rolledout false");
        let cond = Condition::bad(&self.applier, err, reason.clone());
        let now = make_date();
        let data = json!({
            "status": {
                "conditions": {
                    "rolledout": cond
                },
                "summary": {
                    "lastRollout": now,
                    "lastFailureReason": reason,
                    "lastAction": "Rollout",
                }
            }
        });
        self.patch(&data)
    }

    pub fn update_rollout_true(&self) -> Result<ManifestK> {
        debug!("Setting rolledout true");
        let now = make_date();
        let cond = Condition::ok(&self.applier);
        let data = json!({
            "status": {
                "conditions": {
                    "rolledout": cond
                },
                "summary": {
                    "lastRollout": now,
                    "lastSuccessfulRollout": now,
                    "lastFailureReason": null,
                    "lastAction": "Rollout",
                }
            }
        });
        self.patch(&data)
    }
}

fn format_date_diff(old_ts: &str) -> Result<String> {
    use chrono::{Duration, DateTime};
    let last = old_ts.parse::<DateTime<Utc>>()?;
    let diff : Duration = Utc::now() - last;
    let days = diff.num_days();
    let hours = diff.num_hours();
    let mins = diff.num_minutes();
    let diff_fmt = if days >= 1 {
        let plural = if days > 1 { "s" } else { "" };
        format!("{} day{}", days, plural)
    } else if hours >= 1 {
        let plural = if hours > 1 { "s" } else { "" };
        format!("{} hour{}", hours, plural)
    } else {
        let plural = if mins > 1 { "s" } else { "" };
        format!("{} minute{}", mins, plural)
    };
    Ok(diff_fmt)
}

fn format_condition(name: &str, cond: &Condition) -> Result<()> {
    use std::io::{self, Write};
    print!("{}", name);
    let when = format_date_diff(&cond.last_transition)?;
    print!(" {} ago", when);
    if let Some(src) = &cond.source {
        let via = if let Some(url) = &src.url {
            format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", url, src.name)
        } else {
            src.name.clone()
        };
        print!(" via {}", via);
    }
    if cond.status {
        print!(" (Success)");
    } else if let (Some(r), Some(msg)) = (&cond.reason, &cond.message) {
        print!(" ({}: {})", r, msg);
    } else {
        print!(" (Failure)"); // no reason!?
    }
    print!("\n");
    let _ = io::stdout().flush();
    Ok(())
}

use crate::{Config, Region};
/// Entry point for `shipcat status`
pub fn show(svc: &str, conf: &Config, reg: &Region) -> Result<()> {
    use crate::kubectl;
    let mf = shipcat_filebacked::load_manifest(svc, conf, reg)?;
    let crd = Status::new(&mf)?.get()?;

    let md = mf.metadata.clone().expect("need metadata");
    let ver = crd.spec.version.expect("need version");
    let support = md.support.clone().unwrap();
    let link = md.github_link_for_version(&ver);
    // crazy terminal hyperlink escape codes with rust format {} parts:
    let term_repo = format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", md.repo, mf.name.to_uppercase());
    let term_version = format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", link, ver);
    let slack_link = format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", support.link(&conf.slack), *support);

    println!("==> {} is running {}", term_repo, term_version);
    println!("{}", slack_link);
    println!("");

    println!("==> CONDITIONS");
    if let Some(stat) = crd.status {
        let conds = &stat.conditions;
        if let Some(gen) = &conds.generated {
            format_condition("Generated", &gen)?;
        }
        if let Some(app) = &conds.applied {
            format_condition("Applied", &app)?;
        }
        if let Some(ro) = &conds.rolledout {
            format_condition("RolledOut", &ro)?;
        }
    }
    println!("");

    println!("==> RESOURCES");
    println!("{}", kubectl::kpods(&mf)?);
    Ok(())
}
