use std::fs;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Write;

use serde_yaml;

use super::{Result, Manifest, Config, Region};

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

/// Create helm values file for a service
///
/// Requires a completed manifest (with inlined configs)
pub fn values(mf: &Manifest, output: &str) -> Result<()> {
    let encoded = serde_yaml::to_string(&mf)?;
    let pth = Path::new(".").join(output);
    debug!("Writing helm values for {} to {}", mf.name, pth.display());
    let mut f = File::create(&pth)?;
    writeln!(f, "{}", encoded)?;
    debug!("Wrote helm values for {} to {}: \n{}", mf.name, pth.display(), encoded);
    Ok(())
}


/// Analogue of helm template
///
/// Generates helm values to disk, then passes it to helm template
pub fn template(svc: &str, region: &Region, conf: &Config, ver: Option<String>, mock: bool, output: Option<PathBuf>) -> Result<String> {
    let mut mf = if mock {
        shipcat_filebacked::load_manifest(svc, conf, region)?.stub(region)?
    } else {
        shipcat_filebacked::load_manifest(svc, conf, region)?.complete(region)?
    };

    // template or values does not need version - but respect passed in / manifest
    if ver.is_some() {
        // override with set version only if set - respect pin otherwise
        mf.version = ver;
    }
    // sanity verify what we changed (no-shoehorning in illegal versions in rolling envs)
    if let Some(v) = &mf.version {
        region.versioningScheme.verify(&v)?;
    }

    let hfile = format!("{}.helm.gen.yml", svc);
    values(&mf, &hfile)?;

    // helm template with correct params
    let tplvec = vec![
        "template".into(),
        format!("charts/{}", mf.chart.unwrap()),
        "-f".into(),
        hfile.clone(),
    ];
    // NB: this call does NOT need --tiller-namespace (offline call)
    let (tpl, tplerr, success) = hout(tplvec.clone())?;
    if !success {
        warn!("{} stderr: {}", tplvec.join(" "), tplerr);
        bail!("helm template failed");
    }
    if let Some(o) = output {
        let pth = Path::new(".").join(o);
        info!("Writing helm template for {} to {}", svc, pth.display());
        let mut f = File::create(&pth)?;
        writeln!(f, "{}", tpl)?;
        debug!("Wrote helm template for {} to {}: \n{}", svc, pth.display(), tpl);
    } else {
        println!("{}", tpl);
    }
    fs::remove_file(hfile)?;
    Ok(tpl)
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

