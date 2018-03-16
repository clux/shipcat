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
pub fn shell(mf: &Manifest, desiredpod: Option<u32>, cmd: Option<Vec<&str>>) -> Result<()> {

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

        debug!("Shelling into {}", p);
        //kubectl exec -n $ns -it $$pod sh
        let mut execargs = vec![
            "exec".into(),
            "-it".into(),
            p.into(),
        ];
        if ns != "" {
            execargs.push("-n".into());
            execargs.push(ns.clone());
        }
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
