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

pub fn current_context() -> Result<String> {
    let mut res = kout(vec!["config".into(), "current-context".into()])?;
    let len = res.len();
    if res.ends_with('\n') {
        res.truncate(len - 1);
    }
    Ok(res)
}

pub fn current_namespace(ctx: &str) -> Result<String> {
    let res = kout(vec![
        "config".into(),
        "get-contexts".into(),
        ctx.into(),
        "--no-headers".into(),
    ])?;
    if res.contains(ctx) {
        if let Some(ns) = res.split_whitespace().last() {
            return Ok(ns.into());
        }
    }
    bail!("Failed to find default namespace from kube context {}", ctx)
}

fn get_pods(name: &str) -> Result<String> {
    //kubectl get pods -l=app=$* -o jsonpath='{.items[*].metadata.name}'
    let podargs = vec![
        "get".into(),
        "pods".into(),
        format!("-l=app={}", name),
        "-o".into(),
        "jsonpath='{.items[*].metadata.name}'".into(),
    ];
    // TODO: filter out ones not running conditionally - exec wont work with this
    let podsres = kout(podargs)?;
    debug!("Active pods: {:?}", podsres);
    Ok(podsres)
}

pub fn get_broken_pods(name: &str) -> Result<Vec<String>> {
    let podargs = vec![
        "get".into(),
        "pods".into(),
        format!("-l=app={}", name),
        format!("--no-headers"),
    ];
    let podres = kout(podargs)?;
    let mut bpods = vec![];
    for l in podres.lines() {
        if !l.contains("Running") {
            if let Some(p) = l.split(' ').next() {
                warn!("Found pod not running: {}", p);
                bpods.push(p.into());
            }
        }
    }
    Ok(bpods)
}

/// Debug helper when upgrades fail
///
/// Prints log excerpts and events for broken pods.
/// Typically enough to figure out why upgrades broke.
pub fn debug(svc: &str) -> Result<()> {
    let pods = get_broken_pods(&svc)?;
    if pods.is_empty() {
        info!("No broken pods found");
    }
    for pod in pods.clone() {
        warn!("Debugging non-running pod {}", pod);
        warn!("Last 30 log lines:");
        let logvec = vec![
            "logs".into(),
            pod.clone(),
            format!("--tail=30").into(),
        ];
        match kout(logvec) {
            Ok(l) => {
                // TODO: stderr?
                print!("{}\n", l);
            },
            Err(e) => {
                warn!("Failed to get logs from {}: {}", pod, e)
            }
        }
    }

    for pod in pods {
        warn!("Describing events for pod {}", pod);
        let descvec = vec![
            "describe".into(),
            "pod".into(),
            pod.clone()
        ];
        match kout(descvec) {
            Ok(mut o) => {
                if let Some(idx) = o.find("Events:\n") {
                    print!("{}\n", o.split_off(idx))
                }
                else {
                    // Not printing in this case, tons of secrets in here
                    warn!("Unable to find events for pod {}", pod);
                }
            },
            Err(e) => {
                warn!("Failed to describe {}: {}", pod, e)
            }
        }
    }
    Ok(())
}


/// Shell into all pods associated with a service
///
/// Optionally specify the arbitrary pod index from kubectl get pods
pub fn shell(mf: &Manifest, desiredpod: Option<usize>, cmd: Option<Vec<&str>>) -> Result<()> {
    // TODO: kubectl auth can-i create pods/exec
    let podsres = get_pods(&mf.name)?;
    let pods = podsres.split(' ').collect::<Vec<_>>();
    let pnr = desiredpod.unwrap_or(0);
    if let Some(p) = pods.get(pnr) {
        debug!("Shelling into {}", p);
        //kubectl exec -it $pod sh
        let mut execargs = vec![
            "exec".into(),
            "-it".into(),
            p.to_string(),
        ];
        if let Some(cmdu) = cmd.clone() {
            for c in cmdu {
                execargs.push(c.into())
            }
        } else {
            let trybash = vec![
                "exec".into(),
                p.to_string(),
                "which".into(),
                "bash".into(),
            ];
            // kubectl exec $pod which bash
            // returns a non-zero rc if not found generally
              let shexe = match kout(trybash) {
                Ok(o) => {
                    debug!("Got {}", o);
                    "bash".into()
                },
                Err(e) => {
                    warn!("No bash in container, falling back to `sh`");
                    debug!("Error: {}", e);
                    "sh".into()
                }
            };
            execargs.push(shexe);
        }
        kexec(execargs)?;
    } else {
        bail!("Pod {} not found for service {}", pnr, &mf.name);
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use std::env;
    use super::{current_namespace, current_context};

    #[test]
    fn validate_ctx() {
        let kubecfg = env::home_dir().unwrap().join(".kube").join("config");
        if kubecfg.is_file() {
            let ctx = current_context().unwrap();
            assert_eq!(ctx, "dev-uk".to_string());
        }
    }

    #[test]
    fn validate_namespace() {
        let kubecfg = env::home_dir().unwrap().join(".kube").join("config");
        if kubecfg.is_file() {
            let ns = current_namespace("dev-uk").unwrap();
            assert_eq!(ns, "dev".to_string());
        }
    }
}
