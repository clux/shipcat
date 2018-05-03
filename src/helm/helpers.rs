use serde_yaml;

use semver::Version;
use regex::Regex;

use super::RegionDefaults;
use super::{Result, Manifest};


pub fn diff_format(diff: String) -> String {
    let diff_re = Regex::new(r"has changed|^\-|^\+").unwrap();
    // filter out lines that doesn't contain "has changed" or starting with + or -
    diff.split("\n").filter(|l| {
        diff_re.is_match(l)
    }).collect::<Vec<_>>().join("\n")
}


/// Infer a version change diff and extract old version and new version
///
/// Example input:
/// pa-aggregator, Deployment (extensions/v1beta1) has changed:
/// -         image: "quay.io/babylonhealth/pa-aggregator-python:e7c1e5dd5de74b2b5da5eef76eb5bf12bdc2ac19"
/// +         image: "quay.io/babylonhealth/pa-aggregator-python:d4f01f5143643e75d9cc2d5e3221e82a9e1c12e5"
///
/// Output: The two sha1s
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
    let mut out = input.clone();
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

pub fn calculate_wait_time(mf: &Manifest) -> u32 {
    // TODO: need to take into account image size!
    let rcount = mf.replicaCount.unwrap(); // this is set by defaults!
    if let Some(ref hc) = mf.health {
        // wait for at most 2 * bootTime * replicas
        2 * hc.wait * rcount
    } else {
        // sensible guess for boot time
        2 * 30 * rcount
    }
}



/// Values parsed from `helm get values {service}`
///
/// This is the completed manifests including templates, but we only need one key
/// Just parsing this key also makes it more forwards compatible
#[derive(Deserialize)]
struct HelmVals {
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


pub fn infer_fallback_version(service: &str, reg: &RegionDefaults) -> Result<String> {
    // fetch current version from helm
    let imgvec = vec![
        format!("--tiller-namespace={}", reg.namespace),
        "get".into(),
        "values".into(),
        service.into(),
    ];
    debug!("helm {}", imgvec.join(" "));
    match hout(imgvec.clone()) {
        // got a result from helm + rc was 0:
        Ok((vout, verr, true)) => {
            if !verr.is_empty() {
                warn!("{} stderr: {}", imgvec.join(" "), verr);
            }
            // if we got this far, release was found
            // it should work to parse the HelmVals subset of the values:
            let values : HelmVals = serde_yaml::from_str(&vout.to_owned())?;
            Ok(values.version)
        },
        _ => {
            // nothing from helm, fallback to region defaults from config
            Ok(reg.version.clone())
        }
    }
}


pub fn version_validate(mf: &Manifest) -> Result<String> {
    // version MUST be set by main.rs / cluster.rs / whatever.rs before using helm
    // programmer error to not do so; hence the unwrap and backtrace crash
    let ver = mf.version.clone().unwrap();
    let img = mf.image.clone().unwrap();

    // Version sanity: must be full git sha || semver
    let gitre = Regex::new(r"^[0-9a-f\-]{40}$").unwrap();
    if !gitre.is_match(&ver) && Version::parse(&ver).is_err() {
        warn!("Please supply a 40 char git sha version, or a semver version for {}", mf.name);
        if img.contains("quay.io/babylon") {
            // TODO: locked down repos in config ^
            bail!("Floating tag {} cannot be rolled back - not upgrading", ver);
        }
    }
    Ok(ver)
}

#[cfg(test)]
mod tests {
    //use tests::setup;
    use super::{infer_version_change};

    #[test]
    fn version_change_test() {
        let input = "pa-aggregator, Deployment (extensions/v1beta1) has changed:
-         image: \"quay.io/babylonhealth/pa-aggregator-python:e7c1e5dd5de74b2b5da5eef76eb5bf12bdc2ac19\"
+         image: \"quay.io/babylonhealth/pa-aggregator-python:d4f01f5143643e75d9cc2d5e3221e82a9e1c12e5\"";
        let res = infer_version_change(input);
        assert!(res.is_some());
        let (old, new) = res.unwrap();
        assert_eq!(old, "e7c1e5dd5de74b2b5da5eef76eb5bf12bdc2ac19");
        assert_eq!(new, "d4f01f5143643e75d9cc2d5e3221e82a9e1c12e5");
    }
}
