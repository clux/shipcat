use serde_yaml;

use semver::Version;
use regex::Regex;

use super::RegionDefaults;
use super::{Result, Manifest};


pub fn diff_format(diff: String) -> String {
    use regex::Regex;

    let diff_re = Regex::new(r"has changed|^\-|^\+").unwrap();
    // filter out lines that doesn't contain "has changed" or starting with + or -
    diff.split("\n").filter(|l| {
        diff_re.is_match(l)
    }).collect::<Vec<_>>().join("\n")
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

pub fn infer_ci_links() -> Option<String> {
    use std::env;
    use std::process::Command;
    // check jenkins evars first
    if let (Ok(url), Ok(name), Ok(nr)) = (env::var("BUILD_URL"),
                                          env::var("JOB_NAME"),
                                          env::var("BUILD_NUMBER")) {
        Some(format!("{}|{} #{}", url, name, nr))
    } else {
        match Command::new("whoami").output() {
            Ok(s) => {
                let mut out : String = String::from_utf8_lossy(&s.stdout).into();
                let len = out.len();
                if out.ends_with('\n') {
                    out.truncate(len - 1)
                }
                return Some(out)
            }
            Err(e) => {
                warn!("Could not retrieve user from shell {}", e);
                return None
            }
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
