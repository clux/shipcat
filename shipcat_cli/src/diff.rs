use regex::Regex;
use shipcat_definitions::Crd;
use crate::kube;
use crate::helm;
use crate::apply;
use super::{Config, Region, Result};
use std::process::Command;


fn git(args: &[&str]) -> Result<()> {
    debug!("git {}", args.join(" "));
    let s = Command::new("git").args(args).status()?;
    if !s.success() {
        bail!("Subprocess failure from git: {}", s.code().unwrap_or(1001))
    }
    Ok(())
}

/// Fast local git compare of the crd
///
/// Should be pretty safe. Stashes existing work, checks out master, compares,
/// then goes back to previous branch and pops the stash.
///
/// Because this does fiddle with git state while running it is not the default implementation.
pub fn values_vs_git(svc: &str, conf: &Config, region: &Region) -> Result<bool> {
    let aftermf = shipcat_filebacked::load_manifest(&svc, conf, region)?;
    let after = serde_yaml::to_string(&aftermf)?;

    // move git to get before state:
    git(&["checkout", "master", "--quiet"])?;
    let needs_stash = git(&["diff", "--quiet", "--exit-code"]).is_err() || git(&["diff", "--cached", "--quiet", "--exit-code"]).is_err();
    if needs_stash {
        git(&["stash", "--quiet"])?;
    }

    // compute before state
    let beforemf = shipcat_filebacked::load_manifest(&svc, conf, region)?;
    let before = serde_yaml::to_string(&beforemf)?;

    // move git back
    if needs_stash {
        git(&["stash", "pop", "--quiet"])?;
    }
    git(&["checkout", "-", "--quiet"])?;

    // display diff
    shell_diff(&before, &after)
}

/// Fast local git compare of shipcat template
///
/// Because this uses the template in master against local state,
/// we don't resolve secrets for this (would compare equal values anyway).
pub fn template_vs_git(svc: &str, conf: &Config, region: &Region) -> Result<bool> {
    let mock = true; // both would be equivalent vault reads anyway
    let afterpth = Path::new(".").join("after.shipcat.gen.yml");
    let _after = helm::template(&svc, &region, &conf, None, mock, Some(afterpth.clone()))?;

    // move git to get before state:
    git(&["checkout", "master", "--quiet"])?;
    let needs_stash = git(&["diff", "--quiet", "--exit-code"]).is_err() || git(&["diff", "--cached", "--quiet", "--exit-code"]).is_err();
    if needs_stash {
        git(&["stash", "--quiet"])?;
    }

    // compute old state:
    let beforepth = Path::new(".").join("before.shipcat.gen.yml");
    let _before = helm::template(&svc, &region, &conf, None, mock, Some(beforepth.clone()))?;

    // move git back
    if needs_stash {
        git(&["stash", "pop", "--quiet"])?;
    }
    git(&["checkout", "-", "--quiet"])?;

    // display diff
    // doesn't reuse shell_diff because we already have files from direct::template
    let args = ["-u", "before.shipcat.gen.yml", "after.shipcat.gen.yml"];
    debug!("diff {}", args.join(" "));
    let s = Command::new("diff").args(&args).status()?;
    // cleanup
    fs::remove_file(beforepth)?;
    fs::remove_file(afterpth)?;
    Ok(s.success())
}

/// Temporary helm diff wrapper for shipcat::diff
///
/// This will be removed once tiller dependency is removed and 1.13 is everywhere.
pub fn helm_diff(svc: &str, conf: &Config, region: &Region, mock: bool) -> Result<bool> {
    let mfbase = shipcat_filebacked::load_manifest(svc, conf, region)?;
    let mf = if mock {
        mfbase.complete(region)?
    } else {
        mfbase.stub(region)?
    };
    let hfile = format!("{}.helm.gen.yml", svc);
    helm::values(&mf, &hfile)?;
    let diff = match apply::diff_helm(&mf, &hfile) {
        Ok(hdiff) => hdiff,
        Err(e) => {
            warn!("Unable to diff against {}: {}", svc, e);
            None
        },
    };
    if let Some(d) = &diff {
        println!("{}", d);
    }
    let _ = fs::remove_file(&hfile); // try to remove temporary file
    Ok(diff.is_some())
}


use std::path::Path;
use std::fs::{self, File};
use std::io::Write;

/// Diff values using kubectl diff
///
/// Generate crd as we write it and pipe it to `kubectl diff -`
/// Only works on clusters with kubectl 1.13 on the server side, so not available everywhere
pub fn values_vs_kubectl(svc: &str, conf: &Config, region: &Region) -> Result<bool> {
    // Generate crd in a temp file:
    let mf = shipcat_filebacked::load_manifest(svc, conf, region)?;
    let crd = Crd::from(mf);
    let encoded = serde_yaml::to_string(&crd)?;
    let cfile = format!("{}.shipcat.crd.gen.yml", svc);
    let pth = Path::new(".").join(cfile);
    debug!("Writing crd for {} to {}", svc, pth.display());
    let mut f = File::create(&pth)?;
    writeln!(f, "{}", encoded)?;
    // shell out to kubectl:
    let (out, success) = kube::diff(pth.clone(), &region.namespace)?;
    println!("{}", out);
    // cleanup:
    fs::remove_file(pth)?;
    Ok(success)
}

/// Diff using template kubectl diff
///
/// Generate template as we write it and pipe it to `kubectl diff -`
/// Only works on clusters with kubectl 1.13 on the server side, so not available everywhere
pub fn template_vs_kubectl(svc: &str, conf: &Config, region: &Region, mock: bool) -> Result<bool> {
    // Generate template in a temp file:
    let tfile = format!("{}.shipcat.tpl.gen.yml", svc);
    let pth = Path::new(".").join(tfile);
    let version = None; // TODO: override in rolling?
    helm::template(&svc, &region, &conf, version, mock, Some(pth.clone()))?;

    let (out, success) = kube::diff(pth.clone(), &region.namespace)?;
    println!("{}", out);
    // cleanup:
    fs::remove_file(pth)?;
    Ok(success)
}

// Compare using diff(1)
// difference libraries all seemed to be lacking somewhat
fn shell_diff(before: &str, after: &str) -> Result<bool> {
    let beforepth = Path::new(".").join("before.shipcat.gen.yml");
    debug!("Writing before to {}", beforepth.display());
    let mut f = File::create(&beforepth)?;
    writeln!(f, "{}", before)?;

    let afterpth = Path::new(".").join("after.shipcat.gen.yml");
    debug!("Writing after to {}", afterpth.display());
    let mut f = File::create(&afterpth)?;
    writeln!(f, "{}", after)?;

    let args = ["-u", "before.shipcat.gen.yml", "after.shipcat.gen.yml"];
    debug!("diff {}", args.join(" "));
    let s = Command::new("diff").args(&args).status()?;
    // cleanup
    fs::remove_file(beforepth)?;
    fs::remove_file(afterpth)?;

    Ok(s.success())
}

/// Minify diff output from kubectl diff or helm diff
///
/// Trims out non-change lines
pub fn minify(diff: String) -> String {
    let diff_re = Regex::new(r"has changed|^\-|^\+").unwrap();
    // filter out lines that doesn't contain "has changed" or starting with + or -
    diff.split('\n').filter(|l| {
        diff_re.is_match(l)
    }).collect::<Vec<_>>().join("\n")
}

/// Check if a diff contains only version related changes
pub fn is_version_only(diff: &str, vers: (&str, &str)) -> bool {
    let smalldiff = minify(diff.to_string());
    trace!("Checking diff for {:?}", vers);
    for l in smalldiff.lines() {
        // ignore headline for resource type (no actual changes)
        if !l.starts_with('+') && !l.starts_with('-') && l.contains("has changed:") {
            continue;
        }
        // ignore all lines that contain one of the versions
        if l.contains(vers.0) || l.contains(vers.1) {
            continue;
        }
        // any other lines found => not just a version change
        return false;
    }
    true
}

/// Infer a version change diff and extract old version and new version
pub fn infer_version_change(diff: &str) -> Option<(String, String)> {
    let img_re = Regex::new(r"[^:]+:(?P<version>[a-z0-9\.\-]+)").unwrap();
    let res = img_re.captures_iter(diff).map(|cap| {
        cap["version"].to_string()
    }).collect::<Vec<String>>();
    if res.len() >= 2 {
        return Some((res[0].clone(), res[1].clone()));
    }
    None
}

/// Obfuscate a set of secrets from an input string
pub fn obfuscate_secrets(input: String, secrets: Vec<String>) -> String {
    let mut out = input;
    for s in secrets {
        // If your secret is less than 8 characters, we won't obfuscate it
        // Mostly for fear of clashing with other parts of the output,
        // but also because it's an insecure secret anyway
        if s.len() >= 8 {
            out = out.replace(&s, "************");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{infer_version_change, is_version_only};

    #[test]
    fn version_change_test() {
        let input = "pa-aggregator, Deployment (apps/v1) has changed:
-         image: \"quay.io/babylonhealth/pa-aggregator-python:e7c1e5dd5de74b2b5da5eef76eb5bf12bdc2ac19\"
+         image: \"quay.io/babylonhealth/pa-aggregator-python:d4f01f5143643e75d9cc2d5e3221e82a9e1c12e5\"";
        let res = infer_version_change(input);
        assert!(res.is_some());
        let (old, new) = res.unwrap();
        assert_eq!(old, "e7c1e5dd5de74b2b5da5eef76eb5bf12bdc2ac19");
        assert_eq!(new, "d4f01f5143643e75d9cc2d5e3221e82a9e1c12e5");
    }

    #[test]
    fn version_change_semver() {
        let input = "pa-aggregator, Deployment (apps/v1) has changed:
-         image: \"quay.io/babylonhealth/pa-aggregator-python:1.2.3\"
+         image: \"quay.io/babylonhealth/pa-aggregator-python:1.3.0-alpine\"";
        let res = infer_version_change(input);
        assert!(res.is_some());
        let (old, new) = res.unwrap();
        assert_eq!(old, "1.2.3");
        assert_eq!(new, "1.3.0-alpine");
    }

    #[test]
    fn version_diff_test() {
        // simple version change with versions referenced more than once
        let input = "react-ask-frontend, Deployment (apps/v1) has changed:
-         image: \"quay.io/babylonhealth/react-ask-frontend:6418d7cacb7438ddd4e533d78b38902bc7f79e7b\"
+         image: \"quay.io/babylonhealth/react-ask-frontend:d27b5c6f96f05436b236dae112c7c8fcedca4c71\"
-           value: 6418d7cacb7438ddd4e533d78b38902bc7f79e7b
+           value: d27b5c6f96f05436b236dae112c7c8fcedca4c71";

        let res = infer_version_change(input);
        assert!(res.is_some());
        let (old, new) = res.unwrap();
        assert_eq!(old, "6418d7cacb7438ddd4e533d78b38902bc7f79e7b");
        assert_eq!(new, "d27b5c6f96f05436b236dae112c7c8fcedca4c71");
        assert!(is_version_only(input, (&new, &old)));
    }

    #[test]
    fn version_diff_test2() {
        // not just a simple version change
        let input = "react-ask-frontend, Deployment (apps/v1) has changed:
-         image: \"quay.io/babylonhealth/react-ask-frontend:6418d7cacb7438ddd4e533d78b38902bc7f79e7b\"
+         image: \"quay.io/babylonhealth/react-ask-frontend:d27b5c6f96f05436b236dae112c7c8fcedca4c71\"
-           blast: keyremoval";
        let res = infer_version_change(input);
        assert!(res.is_some());
        let (old, new) = res.unwrap();
        assert_eq!(old, "6418d7cacb7438ddd4e533d78b38902bc7f79e7b");
        assert_eq!(new, "d27b5c6f96f05436b236dae112c7c8fcedca4c71");
        assert!(!is_version_only(input, (&new, &old)));
    }

    #[test]
    fn version_diff_test3() {
        // semver version change
        let input = "knowledge-base2-search, Deployment (apps/v1) has changed:
-         image: \"quay.io/babylonhealth/knowledgebase2:1.0.6\"
+         image: \"quay.io/babylonhealth/knowledgebase2:1.0.7\"
-           value: 1.0.6
+           value: 1.0.7";
        let res = infer_version_change(input);
        assert!(res.is_some());
        let (old, new) = res.unwrap();
        assert_eq!(old, "1.0.6");
        assert_eq!(new, "1.0.7");
        assert!(is_version_only(input, (&new, &old)));
    }
}
