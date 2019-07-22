use std::collections::BTreeMap;
use shipcat_definitions::config::Cluster;
use crate::kube;
use super::{Config, Region, Result};
use std::process::Command;

fn find_cluster(clusters: &BTreeMap<String, Cluster>, region_name: &str) -> Option<Cluster> {
    for (_, c) in clusters {
        if c.regions.iter().any(|r| r == region_name) {
            return Some(c.clone());
        }
    }
    None
}


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
        return Ok(valid_ln.contains("EXPIRED"))
    } else {
        debug!("No {} found in tsh status", url);
        Ok(true)
    }
}

pub fn eks_auth(conf: &Config, region: &Region) -> Result<()> {
    let s = Command::new("which").args(vec!["tsh"]).output()?;
    let out = String::from_utf8_lossy(&s.stdout);
    if out.is_empty() {
        bail!("tsh not found. please install tsh --> https://gravitational.com/teleport/download/
Download link for MacOS --> https://get.gravitational.com/teleport-v3.2.6-darwin-amd64-bin.tar.gz
You must install version 3.2.* and not 4.0.0");
    }

    if let Some(cluster) = find_cluster(&conf.clusters, &region.name) {
        if let Some(teleport) = &cluster.teleport {
            if need_teleport_login(&teleport)? {
                let tsh_args = vec![
                    "login".into(),
                    format!("--ttl={}", 60*12), // explicit tsh default
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
                debug!("Reusing active session for {}", teleport);
            }

            // NB: tsh creates a cluster entry in ~/.kube/config named after the url
            // We cannot customize this name the name of this cluster
            let args = vec![
                format!("--cluster={}", &teleport),
                format!("--user={}", &teleport),
                format!("--namespace={}", region.namespace),
            ];
            kube::set_context(&region.name, args)?;
            kube::use_context(&region.name)?;
        } else {
            bail!("Cluster for {} does not have a teleport url", region.name);
        }
    } else {
        bail!("Region {} does not have a cluster", region.name);
    }
    Ok(())
}
