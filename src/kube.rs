use super::{Result, Manifest};
use regex::Regex;

fn kexec(args: Vec<String>) -> Result<()> {
    use std::process::Command;
    debug!("kubectl {}", args.join(" "));
    let s = Command::new("kubectl").args(&args).status()?;
    if !s.success() {
        bail!("Subprocess failure from kubectl: {}", s.code().unwrap_or(1001))
    }
    Ok(())
}
fn kout(args: Vec<String>) -> Result<String> {
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
    let mut res = kout(vec!["config".into(), "current-context".into()]).map_err(|e| {
        error!("Failed to Get kubectl config current-context. Is kubectl installed?");
        e
    })?;
    let len = res.len();
    if res.ends_with('\n') {
        res.truncate(len - 1);
    }
    Ok(res)
}

fn get_pods(mf: &Manifest) -> Result<String> {
    //kubectl get pods -l=app=$* -o jsonpath='{.items[*].metadata.name}'
    let podargs = vec![
        "get".into(),
        "pods".into(),
        format!("-l=app={}", mf.name),
        format!("-n={}", mf.namespace),
        "-o".into(),
        "jsonpath='{.items[*].metadata.name}'".into(),
    ];
    // TODO: filter out ones not running conditionally - exec wont work with this
    let podsres = kout(podargs)?;
    debug!("Active pods: {:?}", podsres);
    Ok(podsres)
}

fn get_broken_pods(mf: &Manifest) -> Result<Vec<String>> {
    let podargs = vec![
        "get".into(),
        "pods".into(),
        format!("-l=app={}", mf.name),
        format!("-n={}", mf.namespace),
        format!("--no-headers"),
    ];
    let podres = kout(podargs)?;
    let mut bpods = vec![];
    let status_re = Regex::new(r" (?P<ready>\d+)/(?P<total>\d+) ").unwrap();
    for l in podres.lines() {
        if !l.contains("Running") {
            if let Some(p) = l.split(' ').next() {
                warn!("Found pod not running: {}", p);
                bpods.push(p.into());
            }
        }
        if let Some(caps) = status_re.captures(l) {
            if &caps["ready"] != &caps["total"] {
                if let Some(p) = l.split(' ').next() {
                    warn!("Found pod with less than necessary containers healthy: {}", p);
                    bpods.push(p.into());
                }
            }
        }
    }
    Ok(bpods)
}

/// Debug helper when upgrades fail
///
/// Prints log excerpts and events for broken pods.
/// Typically enough to figure out why upgrades broke.
pub fn debug(mf: &Manifest) -> Result<()> {
    let pods = get_broken_pods(&mf)?;
    if pods.is_empty() {
        info!("No broken pods found");
    }
    for pod in pods.clone() {
        warn!("Debugging non-running pod {}", pod);
        warn!("Last 30 log lines:");
        let logvec = vec![
            "logs".into(),
            pod.clone(),
            format!("-n={}", mf.namespace),
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
            pod.clone(),
            format!("-n={}", mf.namespace),
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
    let podsres = get_pods(&mf)?;
    let pods = podsres.split(' ').collect::<Vec<_>>();
    let pnr = desiredpod.unwrap_or(0);
    if let Some(p) = pods.get(pnr) {
        debug!("Shelling into {}", p);
        //kubectl exec -it $pod sh
        let mut execargs = vec![
            "exec".into(),
            format!("-n={}", mf.namespace),
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
                format!("-n={}", mf.namespace),
                p.to_string(),
                "which".into(),
                "bash".into(),
            ];
            // kubectl exec $pod which bash
            // returns a non-zero rc if not found generally
              let shexe = match kexec(trybash) {
                Ok(o) => {
                    debug!("Got {:?}", o);
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


/// Port forward a port to localhost
pub fn port_forward(mf: &Manifest, desiredpod: Option<usize>) -> Result<()> {
    // TODO: kubectl auth can-i create something?
    let podsres = get_pods(&mf)?;
    let pods = podsres.split(' ').collect::<Vec<_>>();
    let pnr = desiredpod.unwrap_or(0);
    let port = mf.httpPort.unwrap();
    // first 1024 ports need sudo so avoid that
    let localport = if port <= 1024 { 7777 } else { port };
    if let Some(p) = pods.get(pnr) {
        debug!("Port forwarding kube pod {} to localhost:{}", p, localport);
        //kubectl port-forward $pod localport:httpPort
        let mut pfargs = vec![
            format!("-n={}", mf.namespace),
            "port-forward".into(),
            p.to_string(),
            format!("{}:{}", port, port)
        ];
        kexec(pfargs)?;
    } else {
        bail!("Pod {} not found for service {}", pnr, &mf.name);
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use std::env;
    use super::current_context;

    #[test]
    fn validate_ctx() {
        let kubecfg = env::home_dir().unwrap().join(".kube").join("config");
        if kubecfg.is_file() {
            let ctx = current_context().unwrap();
            assert_eq!(ctx, "dev-uk".to_string());
        }
    }
}
