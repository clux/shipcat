use super::{Result, Manifest};

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

// NB: location not used
// assumed to have been sanity checked before!
pub fn rollout(region: &str, tag: &str, mf: &Manifest) -> Result<()> {
    // further sanity
    let confargs = vec!["config".into(), "current-context".into()];
    kexec(confargs)?;

    let env = mf._namespace.clone();
    let loc = mf._location.clone();
    assert!(region.starts_with(&env));
    assert!(region.ends_with(&loc));

    // TODO: check if access to roll out deployment!

    let mut img = mf.image.clone().unwrap();
    img.tag = Some(tag.into());

    let args = vec![
        "set".into(),
        "image".into(),
        format!("deployment/{}", mf.name),
        format!("{}={}", mf.name, img),
        "-n".into(),
        env.clone(),
    ];
    println!("kubectl {}", args.join(" "));
    kexec(args)?;

    let rollargs = vec![
        "rollout".into(),
        "status".into(),
        format!("deployment/{}", mf.name),
        "-n".into(),
        env.clone(),
    ];
    let podargs = vec![
        "get".into(),
        "pods".into(),
        format!("-l=app={}", mf.name),
        "-n".into(),
        env.into(),
    ];
    match kexec(rollargs.clone()) {
        Err(e) => {
            warn!("Rollout seems to hang - investigating");
            warn!("Got: {} from rollout command", e);
            info!("Checking pod status:");
            kexec(podargs)?;
            bail!("rollout failed to succeed in 5minutes");
        }
        Ok(_) => {
            info!("{}@{} rolled out to {}", mf.name, tag, region);
        }
    };
    Ok(())
}


fn get_pods(name: &str, env: &str) -> Result<String> {
    //kubectl get pods -l=app=$* -n $(ENV) -o jsonpath='{.items[*].metadata.name}'
    let mut podargs = vec![
        "get".into(),
        "pods".into(),
        format!("-l=app={}", name),
        "-o".into(),
        "jsonpath='{.items[*].metadata.name}'".into(),
    ];
    // TODO: filter out ones not running conditionally - exec wont work with this
    if env != "" {
        podargs.push("-n".into());
        podargs.push(env.into());
    }
    let podsres = kout(podargs)?;
    debug!("Active pods: {:?}", podsres);
    Ok(podsres)
}

pub fn shell(mf: &Manifest, desiredpod: Option<u32>) -> Result<()> {
    // TODO: check if access to shell in!

    // region might not be set for this command
    // rely on kubectl context to work it out if unset
    let env = mf._namespace.clone();
    //let loc = mf._location.clone();

    let podsres = get_pods(&mf.name, &env)?;
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
        //kubectl exec -n $(ENV) -it $$pod sh
        let mut execargs = vec![
            "exec".into(),
            "-it".into(),
            p.into(),
            "sh".into(),
        ];
        if env != "" {
            execargs.push("-n".into());
            execargs.push(env.clone());
        }
        kexec(execargs)?;
    }
    Ok(())
}

pub fn logs(mf: &Manifest, desiredpod: Option<u32>) -> Result<()> {
    // TODO: check if access to get logs in!

    // region might not be set for this command
    // rely on kubectl context to work it out if unset
    let env = mf._namespace.clone();
    //let loc = mf._location.clone();

    let podsres = get_pods(&mf.name, &env)?;
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
        if env != "" {
            logargs.push("-n".into());
            logargs.push(env.clone());
        }
        kexec(logargs)?;
    }
    Ok(())
}
