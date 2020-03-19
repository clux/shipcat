use crate::{kubeapi::ShipKube, track::PodSummary, Result};
use k8s_openapi::api::core::v1::Pod;
use shipcat_definitions::status::Condition;
use std::convert::TryFrom;

fn format_condition(cond: &Condition) -> Result<String> {
    let mut s = String::from("");
    match cond.format_last_transition() {
        Ok(when) => s += &format!("{} ago", when),
        Err(e) => warn!("failed to parse timestamp from condition: {}", e),
    }
    if let Some(src) = &cond.source {
        let via = if let Some(url) = &src.url {
            format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", url, src.name)
        } else {
            src.name.clone()
        };
        s += &format!(" via {}", via);
    }
    if cond.status {
        s += " (Success)";
    } else if let (Some(r), Some(msg)) = (&cond.reason, &cond.message) {
        s += &format!(" ({}: {})", r, msg);
    } else {
        s += " (Failure)"; // no reason!?
    }
    Ok(s)
}

fn format_pods(pods: Vec<Pod>) -> Result<()> {
    // NB: podname here is our service limit + rs sha len + pod sha len
    println!(
        "{0:<60} {1:<8} {2:<12} {3:<6} {4:<8} {5:<12}",
        "POD", "VERSION", "STATUS", "READY", "RESTARTS", "AGE"
    );
    for pod in pods {
        let podstate = PodSummary::try_from(pod)?;
        println!("{:?}", podstate);
    }
    Ok(())
}

use crate::{Config, Region};
/// Entry point for `shipcat status`
pub async fn show(svc: &str, conf: &Config, reg: &Region) -> Result<()> {
    let mf = shipcat_filebacked::load_manifest(svc, conf, reg).await?;
    let api = ShipKube::new(&mf).await?;
    let crd = api.get().await?;
    let pod_res = api.get_pods().await;

    let md = mf.metadata.clone().expect("need metadata");
    let ver = crd.spec.version.expect("need version");
    let support = md.support.clone().unwrap();
    let link = md.github_link_for_version(&ver);
    // crazy terminal hyperlink escape codes with rust format {} parts:
    let term_repo = format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", md.repo, mf.name.to_uppercase());
    let term_version = format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", link, ver);
    let slack_link = format!(
        "\x1B]8;;{}\x07{}\x1B]8;;\x07",
        support.link(&conf.slack),
        *support
    );

    let mut printed = false;
    if let Some(stat) = &crd.status {
        if let Some(summary) = &stat.summary {
            if let Some(successver) = &summary.last_successful_rollout_version {
                if successver != &ver {
                    print!("==> {} is requesting {}", term_repo, term_version);
                    print!(" but last successful deploy used {}", successver);
                    println!();
                } else {
                    println!("==> {} is running {}", term_repo, term_version);
                }
                printed = true;
            }
        }
    }
    if !printed {
        println!("==> {} is requesting {}", term_repo, term_version);
    }
    println!("{}", slack_link);
    println!();

    println!("==> CONDITIONS");
    if let Some(stat) = crd.status {
        let conds = &stat.conditions;
        if let Some(gen) = &conds.generated {
            println!("Generated {}", format_condition(gen)?);
        }
        if let Some(app) = &conds.applied {
            println!("Applied {}", format_condition(app)?);
        }
        if let Some(ro) = &conds.rolledout {
            println!("RolledOut {}", format_condition(ro)?);
        }
    }
    println!();

    if let Ok(pods) = pod_res {
        println!("==> RESOURCES");
        let mut pvec = pods.into_iter().collect::<Vec<Pod>>();
        pvec.sort_by_key(|p| {
            p.metadata
                .as_ref()
                .unwrap()
                .creation_timestamp
                .as_ref()
                .unwrap()
                .0
        });
        format_pods(pvec)?;
    }
    Ok(())
}
