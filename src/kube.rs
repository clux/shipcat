use super::{Result, Manifest};

fn kubeout(args: Vec<String>) -> Result<()> {
    use std::process::Command;
    let s = Command::new("kubectl").args(&args).status()?;
    if !s.success() {
        bail!("Subprocess failure from kubectl: {}", s.code().unwrap_or(1001))
    }
    Ok(())
}

// NB: location not used
// assumed to have been sanity checked before!
pub fn rollout(region: &str, tag: &str, mf: &Manifest) -> Result<()> {
    // further sanity
    let confargs = vec!["config".into(), "current-context".into()];
    kubeout(confargs)?;

    let env = mf._namespace.clone();
    let loc = mf._location.clone();
    assert!(region.starts_with(&env));
    assert!(region.ends_with(&loc));

    let mut img = mf.image.clone().unwrap();
    img.tag = Some(tag.into());

    let args = vec![
        "set".into(),
        "image".into(),
        format!("deployment/{}", mf.name.clone().unwrap()),
        format!("{}={}", mf.name.clone().unwrap(), img),
        "-n".into(),
        env.clone(),
    ];
    println!("kubectl {}", args.join(" "));
    kubeout(args)?;

    let rollargs = vec![
        "rollout".into(),
        "status".into(),
        format!("deployment/{}", mf.name.clone().unwrap()),
        "-n".into(),
        env.clone(),
    ];
    let podargs = vec![
        "get".into(),
        "pods".into(),
        format!("-l=app={}", mf.name.clone().unwrap()),
        "-n".into(),
        env.into(),
    ];
    match kubeout(rollargs.clone()) {
        Err(e) => {
            warn!("Rollout seems to hang - investigating");
            warn!("Got: {} from rollout command", e);
            info!("Checking pod status:");
            kubeout(podargs)?;
            bail!("rollout failed to succeed in 5minutes");
        }
        Ok(_) => {
            info!("rollout done!");
        }
    };
    Ok(())
}
