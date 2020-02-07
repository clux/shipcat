use crate::{Result, ErrorKind, Manifest};
use serde_json::json;

use kube::{
    api::{Api, Object, PatchParams, DeleteParams},
    client::APIClient,
};

use shipcat_definitions::status::{make_date, ManifestStatus, Applier, Condition, /*ConditionSummary*/};


/// Client creator
///
/// TODO: embed inside shipcat::apply when needed for other things
fn make_client() -> Result<APIClient> {
    let config = kube::config::incluster_config().or_else(|_| {
        kube::config::load_kube_config()
    }).map_err(ErrorKind::KubeError)?;
    Ok(kube::client::APIClient::new(config))
}

/// Kube Object version of Manifest
///
/// This is the new version of Crd<Manifest> (which will be removed)
type ManifestK = Object<Manifest, ManifestStatus>;

type ManifestMinimalK = Object<MinimalManifest, ManifestStatus>;

/// The few immutable properties that always must exist during upgrades
#[derive(Clone, Serialize, Deserialize)]
pub struct MinimalManifest {
    pub name: String,
    pub version: String,
}

/// Interface for dealing with kubernetes shipcatmanifests
pub struct ShipKube {
    scm: Api<ManifestK>,
    scm_minimal: Api<ManifestMinimalK>,
    applier: Applier,
    name: String,
}

/// Entry points for shipcat::apply
impl ShipKube {
    pub fn new_within(svc: &str, ns: &str) -> Result<Self> {
        // hide the client in here -> Api resource for now (not needed elsewhere)
        let client = make_client()?;
        let scm : Api<ManifestK> = Api::customResource(client.clone(), "shipcatmanifests")
            .group("babylontech.co.uk")
            .within(ns);
        let scm_minimal : Api<ManifestMinimalK> = Api::customResource(client, "shipcatmanifests")
            .group("babylontech.co.uk")
            .within(ns);
        Ok(Self {
            name: svc.to_string(),
            applier: Applier::infer(),
            scm: scm,
            scm_minimal: scm_minimal,
        })
    }
    pub fn new(mf: &Manifest) -> Result<Self> {
        Self::new_within(&mf.name, &mf.namespace)
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
        let o = self.scm.get(&self.name).map_err(ErrorKind::KubeError)?;
        Ok(o)
    }

    /// Minimal CRD fetcher (for upgrades)
    pub fn get_minimal(&self) -> Result<ManifestMinimalK> {
        let o = self.scm_minimal.get(&self.name).map_err(ErrorKind::KubeError)?;
        Ok(o)
    }

    /// Minimal CRD deleter
    pub fn delete(&self) -> Result<()> {
        let dp = DeleteParams::default();
        self.scm_minimal.delete(&self.name, &dp).map_err(ErrorKind::KubeError)?;
        Ok(())
    }


    // ====================================================
    // WARNING : PATCH HELL BELOW
    // ====================================================

    // helper to send a merge patch
    fn patch(&self, data: &serde_json::Value) -> Result<ManifestK> {
        let pp = PatchParams::default();
        let o = self.scm.patch_status(&self.name, &pp, serde_json::to_vec(data)?)
            .map_err(ErrorKind::KubeError)?;
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


fn format_condition(cond: &Condition) -> Result<String> {
    let mut s = String::from("");
    match cond.format_last_transition() {
        Ok(when) => s += &format!("{} ago", when),
        Err(e) => warn!("failed to parse timestamp from condition: {}", e),
    }
    if let Some(src) = &cond.source {
        let via = if let Some(url) = &src.url {
            format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", url, src.name)
        } else {
            src.name.clone()
        };
        s += &format!(" via {}", via);
    }
    if cond.status {
        s += " (Success)";
    } else if let (Some(r), Some(msg)) = (&cond.reason, &cond.message) {
        s += &format!(" ({}: {})", r, msg);
    } else {
        s += " (Failure)"; // no reason!?
    }
    Ok(s)
}

use crate::{Config, Region};
/// Entry point for `shipcat status`
pub fn show(svc: &str, conf: &Config, reg: &Region) -> Result<()> {
    use crate::kubectl;
    let mf = shipcat_filebacked::load_manifest(svc, conf, reg)?;
    let crd = ShipKube::new(&mf)?.get()?;

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
    println!();

    println!("==> CONDITIONS");
    if let Some(stat) = crd.status {
        let conds = &stat.conditions;
        if let Some(gen) = &conds.generated {
            println!("Generated {}", format_condition(gen)?);
        }
        if let Some(app) = &conds.applied {
            println!("Applied {}", format_condition(app)?);
        }
        if let Some(ro) = &conds.rolledout {
            println!("RolledOut {}", format_condition(ro)?);
        }
    }
    println!();

    println!("==> RESOURCES");
    print!("{}", kubectl::kpods(&mf)?);
    Ok(())
}
