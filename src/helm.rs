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
pub fn upgrade(dep: &Deployment) -> Result<()> {
    // TODO: check if access to roll out deployment!

    // region sanity
    let kctx = kout(vec!["config".into(), "current-context".into()])?;
    assert_eq!(format!("{}\n", dep.region), kctx); // TODO: fix newline issues from kout
    if !dep.manifest.regions.contains(&dep.region) {
        bail!("This service cannot be deployed in this region")
    }

    let ns = dep.manifest.namespace.clone();

    // either we deploy with an explicit sha (build triggers from repos)
    let version = if let Some(v) = dep.version.clone() {
        v
    } else {
        // else use the current deployed sha (reconciliation)
        let imgvec = vec![
            "get".into(),
            "deploy".into(),
            "-n".into(),
            ns,
            format!("-l=app={}", dep.service),
            "-o=jsonpath='{$.items[:1].spec.template.spec.containers[:1].image}'".into(),
        ];
        debug!("kubectl {}", imgvec.join(" "));
        let image = kout(imgvec)?;
        let split: Vec<&str> = image.split(':').collect();
        if split.len() != 2 {
            bail!("Invalid image '{}' returned from kubectl for {}", image, dep.service)
        }
        split[1].into() // last element is the tag;
    };
    info!("Using version {}", version);
    if dep.version.is_none() {
        info!("Inferred from current running {}", dep.service);
    }
    // now create helm values
    let file = format!("{}.helm.gen.yml", dep.service);
    generate::helm(dep, Some(file.clone()))?;

    // diff against current running
    //helm diff $* charts/$$(yq -r ".chart" helm.yml) -f helm.yml -q
    let diffvec = vec![
        "diff".into(),
        dep.service.clone(),
        format!("charts/{}", dep.manifest.chart),
        "-f".into(),
        file.clone(),
    ];
    debug!("helm {}", diffvec.join(" "));
    hexec(diffvec)?; // just for logs

    // then upgrade it!
    //helm upgrade $* charts/$$(yq -r ".chart" helm.yml) -f helm.yml -q
    let upgradevec = vec![
        "upgrade".into(),
        dep.service.clone(),
        format!("charts/{}", dep.manifest.chart),
        "-f".into(),
        file
    ];
    debug!("helm {}", upgradevec.join(" "));
    hexec(upgradevec)?;

    Ok(())
}

/// Analogoue of helm template
///
/// Defers to `generate::helm` for now
pub fn template(dep: &Deployment, output: Option<String>) -> Result<String> {
    generate::helm(dep, output)
}
