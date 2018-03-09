use std::fs;

use super::kube::kout;
use super::generate::{self, Deployment};
use super::{Result};

pub fn hexec(args: Vec<String>) -> Result<()> {
    use std::process::Command;
    debug!("helm {}", args.join(" "));
    let s = Command::new("helm").args(&args).status()?;
    if !s.success() {
        bail!("Subprocess failure from helm: {}", s.code().unwrap_or(1001))
    }
    Ok(())
}

fn infer_version(service: &str, ns: &str, image: &str) -> Result<String> {
    // else use the current deployed sha (reconciliation)
    let imgvec = vec![
        "get".into(),
        "deploy".into(),
        "-n".into(),
        ns.into(),
        format!("-l=app={}", service),
        "-o=jsonpath='{$.items[:1].spec.template.spec.containers[:].image}'".into(),
    ];
    // NB: could do containers[:] and search for `img` as well
    debug!("kubectl {}", imgvec.join(" "));
    let imagestr = kout(imgvec)?;
    // first split into a vector of images
    debug!("Found images {}", imagestr);
    for i in imagestr.split(' ') {
        trace!("Looking for {} in {}", image, i);
        if i.contains(image) {
            let split: Vec<&str> = i.split(':').collect();
            if split.len() != 2 {
                bail!("Image '{}' for service {} did not have a tag from kubectl", image, service)
            }
            return Ok(split[1].into()) // last element is the tag;
        }
    }
    bail!("Failed to find {} in spec.containers to infer image", image)
}

/// Upgrade an an existing deployment if needed
///
/// This can be given an explicit semver version (on trigger)
/// or be used be a reconciliation job (in which case the current version is reused).
///
/// This essentially wraps command sequences like:
/// shipcat helm -r {region} {service} template > helm.yml
/// # missing kubectl step to inject previous version into helm.yml optionally
/// helm diff {service} charts/{chartname} -f helm.yml
/// helm upgrade {service} charts/{chartname} -f helm.yml
pub fn upgrade(dep: &Deployment, dryrun: bool) -> Result<()> {
    // TODO: check if access to roll out deployment!

    // region sanity
    let kctx = kout(vec!["config".into(), "current-context".into()])?;
    assert_eq!(format!("{}\n", dep.region), kctx); // TODO: fix newline issues from kout
    if !dep.manifest.regions.contains(&dep.region) {
        bail!("This service cannot be deployed in this region")
    }

    let ns = dep.manifest.namespace.clone();
    let img = dep.manifest.image.clone().unwrap();

    // either we deploy with an explicit sha (build triggers from repos)
    let version = if let Some(v) = dep.version.clone() {
        v
    } else {
        infer_version(&dep.service, &ns, &img)?
    };
    if dep.version.is_none() {
        info!("Using version {} (inferred from kubectl for current running version)", version);
    } else {
        info!("Using default {} version", version);
    }
    // now create helm values
    let file = format!("{}.helm.gen.yml", dep.service);
    generate::helm(dep, Some(file.clone()))?;

    // diff against current running
    // TODO: fix output here! need to ALWAYS see this
    let diffvec = vec![
        "diff".into(),
        dep.service.clone(),
        format!("charts/{}", dep.manifest.chart),
        "-f".into(),
        file.clone(),
        "--set".into(),
        format!("version={}", version),
    ];
    info!("helm {}", diffvec.join(" "));
    hexec(diffvec)?; // just for logs

    if !dryrun {
        // upgrade it using the same command
        let mut upgradevec = vec![
            "upgrade".into(),
            dep.service.clone(),
            format!("charts/{}", dep.manifest.chart),
            "-f".into(),
            file.clone(),
            "--set".into(),
            format!("version={}", version),
        ];
        if let Some(ref hc) = dep.manifest.health {
            // wait for at most 2 * bootTime * replicas
            let waittime = 2 * hc.wait * dep.manifest.replicaCount;
            upgradevec.extend_from_slice(&[
                "--wait".into(),
                format!("--timeout={}", waittime),
            ]);
        }
        info!("helm {}", upgradevec.join(" "));
        hexec(upgradevec)?;
    }
    fs::remove_file(file)?; // remove temporary file
    Ok(())
}

pub fn diff(dep: &Deployment) -> Result<()> {
    upgrade(dep, true)
}

/// Analogoue of helm template
///
/// Defers to `generate::helm` for now
pub fn template(dep: &Deployment, output: Option<String>) -> Result<String> {
    generate::helm(dep, output)
}
