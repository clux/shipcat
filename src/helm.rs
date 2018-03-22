use std::fs;
use serde_yaml;

use std::fs::File;
use std::path::{Path};
use std::io::{self, Write};

use super::slack;
use super::kube;
use super::generate::{self, Deployment};
use super::{Result, Manifest, Config};

// Struct parsed into from `helm get values {service}`
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
pub fn hout(args: Vec<String>) -> Result<String> {
    use std::process::Command;
    debug!("helm {}", args.join(" "));
    let s = Command::new("helm").args(&args).output()?;
    let out : String = String::from_utf8_lossy(&s.stdout).into();
    Ok(out)
}


fn infer_version(service: &str) -> Result<String> {
    // else use the current deployed sha (reconciliation)
    let imgvec = vec![
        "get".into(),
        "values".into(),
        service.into(),
    ];
    debug!("helm {}", imgvec.join(" "));
    let valuestr = hout(imgvec)?.to_string();
    let values : HelmVals = serde_yaml::from_str(&valuestr)?;
    Ok(values.version)
}

fn infer_jenkins_link() -> Option<String> {
    use std::env;
    use std::process::Command;
    if let (Ok(url), Ok(name), Ok(nr)) = (env::var("BUILD_URL"),
                                          env::var("JOB_NAME"),
                                          env::var("BUILD_NUMBER")) {
        Some(format!("{}|{} #{}", url, name, nr))
    } else {
        match Command::new("whoami").output() {
            Ok(s) => {
                let out : String = String::from_utf8_lossy(&s.stdout).into();
                return Some(out)
            }
            Err(e) => {
                warn!("Could not retrieve user from shell {}", e);
                return None
            }
        }
    }
}

fn pre_upgrade_sanity(dep: &Deployment) -> Result<()> {
    // TODO: kubectl auth can-i rollout Deployment

    // region sanity
    let kctx = kube::current_context()?;
    assert_eq!(dep.region, kctx);
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

fn helm_wait_time(mf: &Manifest) -> u32 {
    let rcount = mf.replicaCount.unwrap(); // this is set by defaults!
    if let Some(ref hc) = mf.health {
        // wait for at most 2 * bootTime * replicas
        2 * hc.wait * rcount
    } else {
        // sensible guess for boot time
        2 * 30 * rcount
    }
}

/// Install a deployment first time
pub fn install(dep: &Deployment, conf: &Config) -> Result<()> {
    pre_upgrade_sanity(dep)?;
    let version = dep.version.clone().unwrap_or_else(|| {
        let r = &conf.regions[&dep.region];
        r.clone().defaults.version
    });

    // create helm values
    let file = format!("{}.helm.gen.yml", dep.service);
    let _ = generate::helm(dep, Some(file.clone()), false)?;

    // install
    let mut installvec = vec![
        "install".into(),
        format!("charts/{}", dep.manifest.chart),
        "-f".into(),
        file.clone(),
        format!("--name={}", dep.service.clone()),
        //"--verify".into(), (doesn't work while chart is a directory)
        "--set".into(),
        format!("version={}", version),
    ];

    installvec.extend_from_slice(&[
        "--wait".into(),
        format!("--timeout={}", helm_wait_time(&dep.manifest)),
    ]);
    match hexec(installvec) {
        Err(e) => {
            error!("{}", e);
            return Err(e);
        },
        Ok(_) => {
            slack::send(slack::Message {
                text: format!("installed {} in {}", &dep.service, dep.region),
                color: Some("good".into()),
                link: infer_jenkins_link(),
                ..Default::default()
            })?;
        }
    };
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
pub fn upgrade(dep: &Deployment, dryrun: bool) -> Result<()> {
    pre_upgrade_sanity(dep)?;

    // either we deploy with an explicit sha (build triggers from repos)
    let version = if let Some(v) = dep.version.clone() {
        v
    } else {
        // TODO: this may fail if the service is down
        infer_version(&dep.service)?
    };
    let action = if dep.version.is_none() {
        info!("Using version {} (inferred from current helm revision)", version);
        "reconcile"
    } else {
        info!("Using default {} version", version);
        "update"
    };
    // now create helm values
    let file = format!("{}.helm.gen.yml", dep.service);
    let mf = generate::helm(dep, Some(file.clone()), false)?;

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
    let smalldiff = diff_format(helmdiff.clone());

    if !helmdiff.is_empty() {
        debug!("{}\n", helmdiff); // full diff for logs
        print!("{}\n", smalldiff);
    } else {
        info!("{} is up to date", dep.service);
    }

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
        upgradevec.extend_from_slice(&[
            "--wait".into(),
            format!("--timeout={}", helm_wait_time(&dep.manifest)),
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

/// Create helm values file for a service
///
/// Defers to `generate::helm` for now
pub fn values(dep: &Deployment, output: Option<String>) -> Result<Manifest> {
    generate::helm(dep, output, false)
}


/// Analogoue of helm template
///
/// Generates helm values to disk, then passes it to helm template
pub fn template(dep: &Deployment, output: Option<String>) -> Result<String> {
    let tmpfile = format!("{}.helm.gen.yml", dep.service);
    let _mf = generate::helm(dep, Some(tmpfile.clone()), true)?;

    // helm template with correct params
    let tplvec = vec![
        "template".into(),
        format!("charts/{}", dep.manifest.chart),
        "-f".into(),
        tmpfile.clone(),
    ];
    let tpl = hout(tplvec)?;
    if let Some(o) = output {
        let pth = Path::new(".").join(o);
        info!("Writing helm template for {} to {}", dep.service, pth.display());
        let mut f = File::create(&pth)?;
        write!(f, "{}\n", tpl)?;
        debug!("Wrote helm template for {} to {}: \n{}", dep.service, pth.display(), tpl);
    } else {
        //stdout only
        let _ = io::stdout().write(tpl.as_bytes());
    }
    fs::remove_file(tmpfile)?;
    Ok(tpl)
}
