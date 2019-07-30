use std::fs;

use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Write;

use serde_yaml;

use super::{Manifest, Config, Region};
use super::{Result};
use super::helpers::{hout};

/// Create helm values file for a service
///
/// Requires a completed manifest (with inlined configs)
pub fn values(mf: &Manifest, output: Option<String>) -> Result<()> {
    let encoded = serde_yaml::to_string(&mf)?;
    if let Some(o) = output {
        let pth = Path::new(".").join(o);
        debug!("Writing helm values for {} to {}", mf.name, pth.display());
        let mut f = File::create(&pth)?;
        writeln!(f, "{}", encoded)?;
        debug!("Wrote helm values for {} to {}: \n{}", mf.name, pth.display(), encoded);
    } else {
        println!("{}\n", encoded);
    }
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
    values(&mf, Some(hfile.clone()))?;

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
