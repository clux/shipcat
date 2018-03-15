use std::fs;

use super::slack;
use super::kube::kout;
use super::generate::{self, Deployment};
use super::{Result, Manifest};

pub fn hexec(args: Vec<String>) -> Result<()> {
    use std::process::Command;
    debug!("helm {}", args.join(" "));
    let s = Command::new("helm").args(&args).status()?;
    if !s.success() {
        bail!("Subprocess failure from helm: {}", s.code().unwrap_or(1001))
    }
    Ok(())
}
pub fn hout(args: Vec<String>) -> Result<String> {
    use std::process::Command;
    debug!("helm {}", args.join(" "));
    let s = Command::new("helm").args(&args).output()?;
    let out : String = String::from_utf8_lossy(&s.stdout).into();
    Ok(out)
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

fn infer_jenkins_link() -> Option<String> {
    use std::env;
    if let (Ok(url), Ok(name), Ok(nr)) = (env::var("BUILD_URL"),
                                          env::var("JOB_NAME"),
                                          env::var("BUILD_NUMBER")) {
        Some(format!("{}|{} #{}", url, name, nr))
    } else {
        None
    }
}

fn pre_upgrade_sanity(dep: &Deployment) -> Result<()> {
    // TODO: kubectl auth can-i rollout Deployment

    // region sanity
    let kctx = kout(vec!["config".into(), "current-context".into()])?;
    assert_eq!(format!("{}\n", dep.region), kctx); // TODO: fix newline issues from kout
    if !dep.manifest.regions.contains(&dep.region) {
        bail!("This service cannot be deployed in this region")
    }

    // slack stuff must also be set:
    slack::env_channel()?;
    slack::env_hook_url()?;

    Ok(())
}

fn diff_format(diff: String) -> String {
    use regex::Regex;

    let diff_re = Regex::new(r"has changed|^\-|^\+").unwrap();
    // filter out lines that doesn't contain "has changed" or starting with + or -
    diff.split("\n").filter(|l| {
        diff_re.is_match(l)
    }).collect::<Vec<_>>().join("\n")
}

fn obfuscate_secrets(input: String, secrets: Vec<String>) -> String {
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
    pre_upgrade_sanity(dep)?;

    let ns = dep.manifest.namespace.clone();
    let img = dep.manifest.image.clone().unwrap();

    // either we deploy with an explicit sha (build triggers from repos)
    let version = if let Some(v) = dep.version.clone() {
        v
    } else {
        // TODO: this may fail if the service is down
        infer_version(&dep.service, &ns, &img)?
    };
    let action = if dep.version.is_none() {
        info!("Using version {} (inferred from kubectl for current running version)", version);
        "reconcile"
    } else {
        info!("Using default {} version", version);
        "update"
    };
    // now create helm values
    let file = format!("{}.helm.gen.yml", dep.service);
    let mf = generate::helm(dep, Some(file.clone()))?;

    // diff against current running
    let diffvec = vec![
        "diff".into(),
        "--no-color".into(),
        dep.service.clone(),
        format!("charts/{}", dep.manifest.chart),
        "-f".into(),
        file.clone(),
        "--set".into(),
        format!("version={}", version),
    ];
    info!("helm {}", diffvec.join(" "));
    let helmdiff = obfuscate_secrets(
        hout(diffvec)?,
        mf._decoded_secrets.values().cloned().collect()
    );

    debug!("{}\n", helmdiff); // full diff for logs
    let smalldiff = diff_format(helmdiff.clone());
    print!("{}\n", smalldiff);

    if !dryrun && !helmdiff.is_empty() {
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
        let waittime = if let Some(ref hc) = dep.manifest.health {
            // wait for at most 2 * bootTime * replicas
            2 * hc.wait * dep.manifest.replicaCount
        } else {
            // sensible guess for boot time
            2 * 30 * dep.manifest.replicaCount
        };
        upgradevec.extend_from_slice(&[
            "--wait".into(),
            format!("--timeout={}", waittime),
        ]);
        info!("helm {}", upgradevec.join(" "));
        match hexec(upgradevec) {
            Err(e) => {
                error!("{}", e);
                slack::send(slack::Message {
                    text: format!("failed to {} {} in {}", action, &dep.service, dep.region),
                    color: Some("danger".into()),
                    link: infer_jenkins_link(),
                    code: Some(smalldiff),
                })?;
            },
            Ok(_) => {
                slack::send(slack::Message {
                    text: format!("{}d {} in {}", action, &dep.service, dep.region),
                    color: Some("good".into()),
                    link: infer_jenkins_link(),
                    code: Some(smalldiff),
                })?;
            }
        };

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
pub fn template(dep: &Deployment, output: Option<String>) -> Result<Manifest> {
    generate::helm(dep, output)
}
