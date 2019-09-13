use crate::kubectl;
use super::{Config, Region, Result};
use std::process::Command;

/// Check if teleport expired
fn need_teleport_login(url: &str) -> Result<bool> {
    let args = vec![
        "status".to_string(),
    ]; // tsh status doesn't seem to have a nice filtering or yaml output :(
    // https://github.com/gravitational/teleport/issues/2869
    let s = Command::new("tsh").args(&args).output()?;
    let tsh_out = String::from_utf8_lossy(&s.stdout);
    let lines = tsh_out.lines().collect::<Vec<_>>();
    if let Some(idx) = lines.iter().position(|l| l.contains(url)) {
        let valid_ln = lines[idx+5]; // idx+5 is Valid until line
        debug!("Checking Valid line {}", valid_ln);
        Ok(valid_ln.contains("EXPIRED"))
    } else {
        debug!("No {} found in tsh status", url);
        Ok(true)
    }
}

fn ensure_teleport() -> Result<()> {
    let s = Command::new("which").args(vec!["tsh"]).output()?;
    let out = String::from_utf8_lossy(&s.stdout);
    if out.is_empty() {
        bail!("tsh not found. please install tsh --> https://gravitational.com/teleport/download/
Download link for MacOS --> https://get.gravitational.com/teleport-v3.2.6-darwin-amd64-bin.tar.gz
You must install version 3.2.* and not 4.0.0");
    }
    // TODO: pin teleport url in cluster entry?
    Ok(())
}

/// Login to a region by going through its owning cluster
///
/// This will use teleport to login if a teleport url is set
/// otherwise it assumes you have already set a context with `region.name` externally.
pub fn login(conf: &Config, region: &Region, force: bool) -> Result<()> {
    if let Some(cluster) = conf.find_owning_cluster(&region) {
        if let Some(teleport) = &cluster.teleport {
            ensure_teleport()?;
            let needs_login = need_teleport_login(&teleport)?;
            if force {
                let tsh_state_file = dirs::home_dir()
                    .expect("need a homedir")
                    .join(".tsh")
                    .join(format!("{}.yaml", teleport));
                debug!("Removing {}", tsh_state_file.display());
                std::fs::remove_file(tsh_state_file)?;
            }
            if needs_login || force {
                let tsh_args = vec![
                    "login".into(),
                    // NB: using default TTL here because there might be a hard limit
                    format!("--proxy={url}:443", url = &teleport),
                    "--auth=github".into(),
                ];
                info!("tsh {}", tsh_args.join(" "));
                let s = Command::new("tsh").args(&tsh_args).output()?;
                let out = String::from_utf8_lossy(&s.stdout);
                let err = String::from_utf8_lossy(&s.stderr);
                if !out.is_empty() {
                    debug!("{}", out);
                }
                if !s.status.success() {
                    bail!("tsh login: {}", err);
                }
            } else {
                info!("Reusing active session for {}", teleport);
            }

            // NB: tsh creates a cluster entry in ~/.kube/config named after the url
            // We cannot customize this name the name of this cluster
            let args = vec![
                format!("--cluster={}", &teleport),
                format!("--user={}", &teleport),
                format!("--namespace={}", region.namespace),
            ];
            kubectl::set_context(&region.name, args)?;
            kubectl::use_context(&region.name)?;
        } else {
            // We assume there's an external way to for users to create kube contexts
            // if not teleport url is set on the owning cluster.
            info!("Reusing {} context for non-teleport region {}", region.cluster, region.name);
            kubectl::use_context(&region.cluster)?;
        }
    } else {
        bail!("Region {} does not have a cluster", region.name);
    }
    Ok(())
}
