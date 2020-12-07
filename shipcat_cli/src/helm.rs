use regex::Regex;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tokio::{
    fs::{self, File},
    prelude::*,
    process::Command,
};

use super::Result;
use shipcat_definitions::{Manifest, ReconciliationMode, Region};

pub fn hexists() -> Result<()> {
    if which::which("helm").is_err() {
        bail!("helm executable not found!");
    }
    Ok(())
}

pub async fn hexec(args: Vec<String>) -> Result<()> {
    debug!("helm {}", args.join(" "));
    hexists()?;
    let s = Command::new("helm").args(&args).status().await?;
    if !s.success() {
        bail!("Subprocess failure from helm: {}", s.code().unwrap_or(1001))
    }
    Ok(())
}
pub async fn hout(args: Vec<String>) -> Result<(String, String, bool)> {
    debug!("helm {}", args.join(" "));
    hexists()?;
    let s = Command::new("helm").args(&args).output().await?;
    let out: String = String::from_utf8_lossy(&s.stdout).into();
    let err: String = String::from_utf8_lossy(&s.stderr).into();
    Ok((out, err, s.status.success()))
}

pub async fn clone_chart(repo_url: &str) -> Result<(String, String, bool)> {
    debug!("git clone {}", repo_url);
    let path = format!("{}{}", "charts/", repo_url);
    if Path::new(&path).exists() {
        debug!("Repo already cloned {}", repo_url);
        Ok(("".to_owned(), "".to_owned(), true))
    } else {
        let re = Regex::new(r"(^git@.*)\?ref=(.*)").unwrap();
        if !re.is_match(repo_url) {
            Ok((
                "".to_owned(),
                "Failed to parse git url, please check it contains a valid ref".to_owned(),
                false,
            ))
        } else {
            let captures = re.captures(repo_url).unwrap();
            let repo = captures.get(1).map_or("", |m| m.as_str());
            let tag = captures.get(2).map_or("", |m| m.as_str());
            let args = ["clone", "-b", &tag, &repo, &path];
            debug!("git clone {}", args.join(" "));
            let s = Command::new("git").args(&args).output().await?;
            let out: String = String::from_utf8_lossy(&s.stdout).into();
            let err: String = String::from_utf8_lossy(&s.stderr).into();
            Ok((out, err, s.status.success()))
        }
    }
}

/// Create helm values file for a service
///
/// Requires a completed manifest (with inlined configs)
pub async fn values(mf: &Manifest, output: &str) -> Result<()> {
    let encoded = serde_yaml::to_string(&mf)?;
    let pth = Path::new(".").join(output);
    debug!("Writing helm values for {} to {}", mf.name, pth.display());
    let mut f = File::create(&pth).await?;
    f.write_all(&encoded.as_bytes()).await?;
    f.sync_data().await?;
    debug!(
        "Wrote helm values for {} to {}: \n{}",
        mf.name,
        pth.display(),
        encoded
    );
    Ok(())
}

/// Analogue of helm template
///
/// Generates helm values to disk, then passes it to helm template
pub async fn template(mf: &Manifest, output: Option<PathBuf>) -> Result<String> {
    let hfile = format!("{}.helm.gen.yml", mf.name);
    values(&mf, &hfile).await?;

    let chart = mf.chart.clone().unwrap();
    if chart.starts_with("git@") {
        let (_tpl, tplerr, success) = clone_chart(&chart).await?;
        if !success {
            warn!("{} stderr: {}", chart, tplerr);
            bail!("helm failed to fetch template");
        }
    }
    // helm template with correct params
    let tplvec = vec![
        "template".into(),
        format!("charts/{}", mf.chart.clone().unwrap()),
        "-f".into(),
        hfile.clone(),
    ];
    // NB: this call does NOT need --tiller-namespace (offline call)
    let (tpl, tplerr, success) = hout(tplvec.clone()).await?;
    if !success {
        warn!("{} stderr: {}", tplvec.join(" "), tplerr);
        bail!("helm template failed");
    }
    if let Some(o) = &output {
        let pth = Path::new(".").join(o);
        debug!("Writing helm template for {} to {}", mf.name, pth.display());
        let mut f = File::create(&pth).await?;
        f.write_all(&tpl.as_bytes()).await?;
        f.sync_data().await?;
        debug!(
            "Wrote helm template for {} to {}: \n{}",
            mf.name,
            pth.display(),
            tpl
        );
        if let Err(e) = fs::remove_file(&hfile).await {
            warn!("Failed to delete file: {} {}", hfile, e);
        }
    }
    Ok(tpl)
}

/// Helper to validate the assumption of the charts
///
/// This is an addon to checks done through `kubeval`.
/// We don't validate kubernetes schemas in here, but we do validate consistency of:
/// - labels: app.kubernetes.io/name, app.kubernetes.io/version, app.kubernetes.io/managed-by
/// - ownerReferences (need ShipcatManifest, !controller, uid propagated, name correct)
pub fn template_check(mf: &Manifest, reg: &Region, skipped: &[String], tpl: &str) -> Result<()> {
    let mut invalids = vec![];
    for to in tpl.split("---") {
        let kind = match serde_yaml::from_str::<PartialObject>(&to) {
            Err(_) => {
                trace!("Skipping partial without kind: {}", to);
                continue;
            }
            Ok(o) => {
                debug!("Checking: {}", o.kind);
                o.kind
            }
        };
        let obj: KubeObject = match serde_yaml::from_str(to) {
            Err(e) => {
                warn!("Invalid KubeObject: {}: {}", kind, e);
                continue;
            }
            Ok(o) => o,
        };
        let name = &obj
            .metadata
            .name
            .clone()
            .unwrap_or_else(|| format!("unset metadata.name from {}", kind));

        let tiller_ok = check_no_tiller_refs(&kind, &obj)?;
        let ok = match reg.reconciliationMode {
            ReconciliationMode::CrdOwned => {
                let owner_ok = check_owner_refs(mf, &kind, &obj)?;
                let labels_ok = check_labels(mf, &kind, skipped, &obj)?;
                labels_ok && owner_ok
            }
        } && tiller_ok;
        if !ok {
            invalids.push(format!("{} {{ {} }}", kind, name));
        }
    }
    if !invalids.is_empty() {
        bail!("Invalid objects: {:?}", invalids);
    }
    Ok(())
}

use kube::api::{ObjectMeta, TypeMeta};
#[derive(Deserialize)]
struct PartialObject {
    kind: String,
}

#[derive(Deserialize)]
struct KubeObject {
    #[serde(flatten)]
    _types: TypeMeta,
    metadata: ObjectMeta,
}

fn check_labels(mf: &Manifest, kind: &str, skipped: &[String], obj: &KubeObject) -> Result<bool> {
    let mut success = true;
    let labels = &obj.metadata.labels.clone().unwrap_or_else(BTreeMap::new);
    match labels.get("app.kubernetes.io/name") {
        Some(n) => {
            if n == &mf.name {
                debug!("{}: valid app.kubernetes.io/name label {}", kind, n)
            } else {
                success = false;
                warn!("{}: invalid app.kubernetes.io/name label {}", kind, n)
            }
        }
        None => {
            success = false;
            warn!("{}: missing app.kubernetes.io/name label", kind);
        }
    };
    match labels.get("app.kubernetes.io/managed-by") {
        Some(n) => {
            if n == "shipcat" {
                debug!("{}: valid app.kubernetes.io/managed-by label {}", kind, n)
            } else {
                success = false;
                warn!("{}: invalid app.kubernetes.io/managed-by label {}", kind, n)
            }
        }
        None => {
            success = false;
            warn!("{}: missing app.kubernetes.io/managed-by label", kind);
        }
    };
    // If the object doesn't get injected into the Deployment automatically
    // then it ought to have the standard version property.
    // If it changes, we should not lie about it changing (Secret + CM didn't really change)
    if !["Secret", "ConfigMap"].contains(&kind) && !skipped.contains(&kind.to_string()) {
        if let Some(v) = &mf.version {
            match labels.get("app.kubernetes.io/version") {
                Some(n) => {
                    if n == v {
                        debug!("{}: valid app.kubernetes.io/version label {}", kind, n)
                    } else {
                        success = false;
                        warn!("{}: invalid app.kubernetes.io/version label {}", kind, n)
                    }
                }
                None => {
                    success = false;
                    warn!("{}: missing app.kubernetes.io/version label", kind);
                }
            };
        }
    }
    Ok(success)
}

fn check_owner_refs(mf: &Manifest, kind: &str, obj: &KubeObject) -> Result<bool> {
    let mut success = true;
    // First ownerReferences must be ShipcatManifest
    match obj.metadata.owner_references.clone().unwrap_or_default().first() {
        Some(or) => {
            if or.kind == "ShipcatManifest" && or.controller != Some(true) && or.name == mf.name {
                debug!("{}: valid ownerReference for {}", kind, or.kind);
            } else {
                success = false;
                warn!(
                    "{}: invalid ownerReference for {}",
                    kind,
                    serde_yaml::to_string(or)?
                );
            }
            if let Some(uid) = &mf.uid {
                if uid == &or.uid {
                    debug!("{}: valid uid in ownerReference for {}", kind, or.uid)
                } else {
                    success = false;
                    warn!("{}: invalid uid in ownerReference for {}", kind, or.uid)
                }
            }
        }
        None => {
            success = false;
            warn!("{}: missing ownerReferences", kind);
        }
    }
    Ok(success)
}

// charts should not reference tiller
fn check_no_tiller_refs(kind: &str, obj: &KubeObject) -> Result<bool> {
    let mut success = true;
    let labels = &obj
        .metadata
        .labels
        .as_ref()
        .unwrap_or_else(|| panic!("kind {} has labels", kind));
    for key in ["chart", "heritage", "release"].iter() {
        match labels.get(&key.to_string()) {
            Some(n) => {
                success = false;
                warn!("{}: {} label {} for tiller not supported", kind, key, n);
            }
            None => {
                debug!("{}: {} label unset", kind, key);
            }
        };
    }
    Ok(success)
}
