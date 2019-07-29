use serde_yaml;

use regex::Regex;
use super::{Result};


pub fn diff_format(diff: String) -> String {
    let diff_re = Regex::new(r"has changed|^\-|^\+").unwrap();
    // filter out lines that doesn't contain "has changed" or starting with + or -
    diff.split('\n').filter(|l| {
        diff_re.is_match(l)
    }).collect::<Vec<_>>().join("\n")
}

pub fn diff_is_version_only(diff: &str, vers: (&str, &str)) -> bool {
    let smalldiff = diff_format(diff.to_string());
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

/// Dump struct with a version field
///
/// Parsed from either:
/// - `helm get values {service}`
/// - `kubectl get shipcatmanifest -oyaml {service}`
#[derive(Deserialize)]
struct DumbVersion {
    version: String,
}

pub fn hexec(args: Vec<String>) -> Result<()> {
    use std::process::Command;
    debug!("helm {}", args.join(" "));
    let s = Command::new("helm").args(&args).status()?;
    if !s.success() {
        bail!("Subprocess failure from helm: {}", s.code().unwrap_or(1001))
    }
    Ok(())
}
pub fn hout(args: Vec<String>) -> Result<(String, String, bool)> {
    use std::process::Command;
    debug!("helm {}", args.join(" "));
    let s = Command::new("helm").args(&args).output()?;
    let out : String = String::from_utf8_lossy(&s.stdout).into();
    let err : String = String::from_utf8_lossy(&s.stderr).into();
    Ok((out, err, s.status.success()))
}

use std::collections::HashSet;
/// Find all services in a given namespace
///
/// Used to warn (for now) when services run that are not in Manifest::available()
/// This is an experimental warning only function.
/// It is possible that `helm ls -q` is still unreliable.
pub fn find_redundant_services(ns: &str, svcs: &[String]) -> Result<Vec<String>> {
    let requested: HashSet<_> = svcs.iter().cloned().collect();

    let lsargs = vec![
        format!("--tiller-namespace={}", ns),
        "ls".into(),
        "-q".into()
    ];
    debug!("helm {}", lsargs.join(" "));
    let found : HashSet<_> = match hout(lsargs.clone()) {
        Ok((vout, verr, true)) => {
            if !verr.is_empty() {
                warn!("helm {} stderr: {}",  lsargs.join(" "), verr);
            }
            // we should have a helm ls -q output:
            vout.lines().into_iter().map(String::from).collect()
        }
        _ => {
            bail!("No services found in {} tiller", ns)
        }
    };
    let excess : HashSet<_> = found.difference(&requested).collect();
    if !excess.is_empty() {
        warn!("Found extraneous helm services: {:?}", excess);
    } else {
        debug!("No excess manifests found");
    }
    Ok(excess.into_iter().cloned().collect())
}

/// Fetch current version from helm get values
pub fn infer_fallback_version(service: &str, ns: &str) -> Result<String> {
    let args = vec![
        format!("--tiller-namespace={}", ns),
        "get".into(),
        "values".into(),
        service.into(),
    ];
    match hout(args.clone()) {
        // got a result from helm + rc was 0:
        Ok((vout, verr, true)) => {
            if !verr.is_empty() {
                warn!("{} stderr: {}", args.join(" "), verr);
            }
            // if we got this far, release was found
            // it should work to parse the HelmVals subset of the values:
            let values : DumbVersion = serde_yaml::from_str(&vout.to_owned())?;
            Ok(values.version)
        },
        _ => {
            // nothing from helm
            bail!("Service {} not found in in {} tiller", service, ns);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{infer_version_change, diff_is_version_only};

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
        assert!(diff_is_version_only(input, (&new, &old)));
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
        assert!(!diff_is_version_only(input, (&new, &old)));
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
        assert!(diff_is_version_only(input, (&new, &old)));
    }
}
