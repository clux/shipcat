use std::fs;
use serde_yaml;

use std::fs::File;
use std::path::{Path};
use std::io::{self, Write};

use super::slack;
use super::generate::{self, Deployment};
use super::{Result, Manifest};
use super::config::{RegionDefaults, Config};

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


pub fn infer_version(service: &str, reg: &RegionDefaults) -> Result<String> {
    // fetch current version from helm
    let imgvec = vec![
        "get".into(),
        "values".into(),
        service.into(),
    ];
    debug!("helm {}", imgvec.join(" "));
    if let Ok(vstr) = hout(imgvec) {
        // if we got this far, release was found
        // it should work to parse the HelmVals subset of the values:
        let values : HelmVals = serde_yaml::from_str(&vstr.to_owned())?;
        return Ok(values.version)
    }
    // nothing from helm, fallback to region defaults from config
    Ok(reg.version.clone())
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

fn pre_upgrade_sanity() -> Result<()> {
    // TODO: kubectl auth can-i rollout Deployment

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
pub fn install(mf: &Manifest, hfile: &str) -> Result<()> {
    pre_upgrade_sanity()?;
    let ver = mf.version.clone().unwrap(); // must be set outside

    // install
    let mut installvec = vec![
        "install".into(),
        format!("charts/{}", mf.chart),
        "-f".into(),
        hfile.into(),
        format!("--name={}", mf.name.clone()),
        //"--verify".into(), (doesn't work while chart is a directory)
        "--set".into(),
        format!("version={}", ver),
    ];

    installvec.extend_from_slice(&[
        "--wait".into(),
        format!("--timeout={}", helm_wait_time(&mf)),
    ]);
    info!("helm {}", installvec.join(" "));
    match hexec(installvec) {
        Err(e) => {
            error!("{}", e);
            return Err(e);
        },
        Ok(_) => {
            slack::send(slack::Message {
                text: format!("installed {} in {}", &mf.name.clone(), &mf._region),
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
pub fn upgrade(mf: &Manifest, hfile: &str, dryrun: bool) -> Result<(Manifest, String)> {
    pre_upgrade_sanity()?;
    let ver = mf.version.clone().unwrap(); // must be set outside
    // diff against current running
    let diffvec = vec![
        "diff".into(),
        "--no-color".into(),
        mf.name.clone(),
        format!("charts/{}", mf.chart),
        "-f".into(),
        hfile.into(),
        "--set".into(),
        format!("version={}", ver),
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
        info!("{} is up to date", mf.name);
    }

    if !dryrun && !helmdiff.is_empty() {
        // upgrade it using the same command
        let mut upgradevec = vec![
            "upgrade".into(),
            mf.name.clone(),
            format!("charts/{}", mf.chart),
            "-f".into(),
            hfile.into(),
            "--set".into(),
            format!("version={}", ver),
        ];
        upgradevec.extend_from_slice(&[
            "--wait".into(),
            format!("--timeout={}", helm_wait_time(mf)),
        ]);
        info!("helm {}", upgradevec.join(" "));
        match hexec(upgradevec) {
            Err(e) => {
                error!("{}", e);
                slack::send(slack::Message {
                    text: format!("failed to update {} in {}", &mf.name, &mf._region),
                    color: Some("danger".into()),
                    link: infer_jenkins_link(),
                    code: Some(smalldiff),
                })?;
                return Err(e);
            },
            Ok(_) => {
                slack::send(slack::Message {
                    text: format!("updated {} in {}", &mf.name, &mf._region),
                    color: Some("good".into()),
                    link: infer_jenkins_link(),
                    code: Some(smalldiff),
                })?;
            }
        };
    }
    Ok((mf.clone(), helmdiff))
}


pub fn diff(mf: &Manifest, hfile: &str) -> Result<(Manifest, String)> {
    upgrade(mf, hfile, true)
}

/// Create helm values file for a service
///
/// Defers to `generate::helm` for now
pub fn values(dep: &Deployment, output: Option<String>, silent: bool) -> Result<Manifest> {
    generate::helm(dep, output, silent)
}


/// Analogue of helm template
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

/// Experimental reconcile that is parallelised
///
/// This still uses helm wait, but it does multiple services at a time.
pub fn reconcile_cluster(conf: &Config, region: String) -> Result<()> {
    use super::vault;
    use super::template;
    let services = Manifest::available()?;
    let mut manifests = vec![];
    for svc in services {
        debug!("Scanning service {:?}", svc);
        let mf = Manifest::basic(&svc, conf, None)?;
        if !mf.disabled && mf.regions.contains(&region) {
            // need a tera per service (special folder handling)
            let tera = template::init(&svc)?;
            let v = vault::Vault::default()?;
            let mut compmf = Manifest::completed(&region, &conf, &svc, Some(v))?;
            let regdefaults = conf.regions.get(&region).unwrap().defaults.clone();
            compmf.version = Some(infer_version(&svc, &regdefaults)?);
            let dep = generate::Deployment {
                service: svc.into(),
                region: region.clone(),
                manifest: compmf,
                render: Box::new(move |tmpl, context| {
                    template::render(&tera, tmpl, context)
                }),
            };
            // create all the values first
            let hfile = format!("{}.helm.gen.yml", dep.service);
            let mfrender = values(&dep, Some(hfile.clone()), false)?;
            manifests.push(mfrender);
        }
    }
    use threadpool::ThreadPool;
    use std::sync::mpsc::channel;

    let n_workers = 8;
    let n_jobs = manifests.len();
    let pool = ThreadPool::new(n_workers);
    info!("Reconciling {} jobs using {} workers", n_jobs, n_workers);

    let (tx, rx) = channel();
    for mf in manifests {
        let tx = tx.clone();
        pool.execute(move|| {
            // Currently this is diff only as it's new!
            let dryrun = true;
            let hfile = format!("{}.helm.gen.yml", mf.name); // as above
            let res = upgrade(&mf, &hfile, dryrun);
            tx.send(res).expect("channel will be there waiting for the pool");
        });
    }
    let _ = rx.iter().take(n_jobs).map(|r| {
        match &r {
            &Ok((ref mf, _)) => info!("Diffed {}", mf.name), // TODO: s/Diffed/Reconciled once !dryrun
            &Err(ref e) => error!("Failed to reconcile {}", e)
        }
        r
    }).collect::<Vec<_>>();

    Ok(())
}
