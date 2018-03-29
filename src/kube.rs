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
    // TODO: sanity check regions in allowed regions first
    Ok(res)
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


/// Shell into all pods associated with a service
///
/// Optionally specify the arbitrary pod index from kubectl get pods
pub fn shell(mf: &Manifest, desiredpod: Option<u32>, cmd: Option<Vec<&str>>) -> Result<()> {
    // TODO: kubectl auth can-i create pods/exec

    let podsres = get_pods(&mf.name)?;
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

        debug!("Shelling into {}", p);
        //kubectl exec -n $ns -it $$pod sh
        let mut execargs = vec![
            "exec".into(),
            "-it".into(),
            p.into(),
        ];
        if let Some(cmdu) = cmd.clone() {
            for c in cmdu {
                execargs.push(c.into())
            }
        } else {
            execargs.push("sh".into())
        }

        kexec(execargs)?;
    }
    Ok(())
}

/// Get the logs for pods associated with a service
///
/// Optionally specify the arbitrary pod index from kubectl get pods
pub fn logs(mf: &Manifest, desiredpod: Option<u32>) -> Result<()> {
    // TODO: kubectl auth can-i get,list pods/logs


    let podsres = get_pods(&mf.name)?;
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
        let logargs = vec![
            "logs".into(),
            p.into(),
        ];
        kexec(logargs)?;
    }
    Ok(())
}
