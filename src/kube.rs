use super::{Result, Manifest};

pub fn kexec(args: Vec<String>) -> Result<()> {
    use std::process::Command;
    debug!("kubectl {}", args.join(" "));
    let s = Command::new("kubectl").args(&args).status()?;
    if !s.success() {
        bail!("Subprocess failure from kubectl: {}", s.code().unwrap_or(1001))
    }
    Ok(())
}
pub fn kout(args: Vec<String>) -> Result<String> {
    use std::process::Command;
    debug!("kubectl {}", args.join(" "));
    let s = Command::new("kubectl").args(&args).output()?;
    let out : String = String::from_utf8_lossy(&s.stdout).into();
    // kubectl keeps returning opening and closing apostrophes - strip them:
    if out.len() > 2 && out.chars().next() == Some('\'') {
        let res = out.split('\'').collect::<Vec<_>>()[1];
        return Ok(res.into());
    }
    Ok(out)
}

/// Rollout an image update to an existing deployment
///
/// Deprecated.
/// This kurrently uses kubectl rollout set image under the hood.
/// This will be replaced by `helm install` in the future
pub fn rollout(region: &str, tag: &str, mf: &Manifest) -> Result<()> {
    // further sanity
    let confargs = vec!["config".into(), "current-context".into()];
    kexec(confargs)?;

    let ns = mf.namespace.clone();
    // TODO: check if access to roll out deployment!

    let img = format!("{}:{}", mf.image.clone().unwrap(), mf.version.clone().unwrap());

    let args = vec![
        "set".into(),
        "image".into(),
        format!("deployment/{}", mf.name),
        format!("{}={}", mf.name, img),
        "-n".into(),
        ns.clone(),
    ];
    println!("kubectl {}", args.join(" "));
    kexec(args)?;

    let rollargs = vec![
        "rollout".into(),
        "status".into(),
        format!("deployment/{}", mf.name),
        "-n".into(),
        ns.clone(),
    ];
    // simple check for routout status first
    match kexec(rollargs.clone()) {
        Err(e) => {
            warn!("Rollout seems to hang - investigating");
            warn!("Got: {} from rollout command", e);
            info!("Checking pod status:");
            let podargs = vec![
                "get".into(),
                "pods".into(),
                format!("-l=app={}", mf.name),
                "-n".into(),
                ns.into(),
            ];
            kexec(podargs)?;
            bail!("rollout failed to succeed in 5minutes");
        }
        Ok(_) => {
            info!("{}@{} rolled out to {}", mf.name, tag, region);
        }
    };
    Ok(())
}


fn get_pods(name: &str, ns: &str) -> Result<String> {
    //kubectl get pods -l=app=$* -n $ns -o jsonpath='{.items[*].metadata.name}'
    let mut podargs = vec![
        "get".into(),
        "pods".into(),
        format!("-l=app={}", name),
        "-o".into(),
        "jsonpath='{.items[*].metadata.name}'".into(),
    ];
    // TODO: filter out ones not running conditionally - exec wont work with this
    if ns != "" {
        podargs.push("-n".into());
        podargs.push(ns.into());
    }
    let podsres = kout(podargs)?;
    debug!("Active pods: {:?}", podsres);
    Ok(podsres)
}

/// Shell into all pods associated with a service
///
/// Optionally specify the arbitrary pod index from kubectl get pods
pub fn shell(mf: &Manifest, desiredpod: Option<u32>) -> Result<()> {
    // TODO: check if access to shell in!

    // region might not be set for this command
    // rely on kubectl context to work it out if unset
    let ns = mf.namespace.clone();

    let podsres = get_pods(&mf.name, &ns)?;
    let pods = podsres.split(' ');

    let mut num = 0;

    for p in pods {
        num += 1;
        if let Some(pnr) = desiredpod {
            if pnr != num {
                trace!("Skipping pod {}", num);
                continue;
            }
        }

        info!("Shelling into {}", p);
        //kubectl exec -n $ns -it $$pod sh
        let mut execargs = vec![
            "exec".into(),
            "-it".into(),
            p.into(),
            "sh".into(),
        ];
        if ns != "" {
            execargs.push("-n".into());
            execargs.push(ns.clone());
        }
        kexec(execargs)?;
    }
    Ok(())
}

/// Get the logs for pods associated with a service
///
/// Optionally specify the arbitrary pod index from kubectl get pods
pub fn logs(mf: &Manifest, desiredpod: Option<u32>) -> Result<()> {
    // TODO: check if access to get logs in!

    // region might not be set for this command
    // rely on kubectl context to work it out if unset
    let ns = mf.namespace.clone();

    let podsres = get_pods(&mf.name, &ns)?;
    let pods = podsres.split(' ');

    let mut num = 0;

    for p in pods {
        num += 1;
        if let Some(pnr) = desiredpod {
            if pnr != num {
                trace!("Skipping pod {}", num);
                continue;
            }
        }

        info!("Logs for {}", p);
        //kubectl logs -n $(ENV) $$pod
        let mut logargs = vec![
            "logs".into(),
            p.into(),
        ];
        if ns != "" {
            logargs.push("-n".into());
            logargs.push(ns.clone());
        }
        kexec(logargs)?;
    }
    Ok(())
}
