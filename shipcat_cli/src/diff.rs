use crate::kube;
use super::{Config, Region, Result};
use shipcat_definitions::Crd;
use std::process::Command;


fn git(args: &[&str]) -> Result<()> {
    debug!("git {}", args.join(" "));
    let s = Command::new("git").args(args).status()?;
    if !s.success() {
        bail!("Subprocess failure from git: {}", s.code().unwrap_or(1001))
    }
    Ok(())
}

/// Fast local git compare of the crd
///
/// Should be pretty safe. Stashes existing work, checks out master, compares,
/// then goes back to previous branch and pops the stash.
///
/// Because this does fiddle with git state while running it is not the default implementation.
pub fn values_vs_git(svc: &str, conf: &Config, region: &Region) -> Result<bool> {
    let aftermf = shipcat_filebacked::load_manifest(&svc, conf, region)?;
    let after = serde_yaml::to_string(&aftermf)?;

    // move git to get before state:
    git(&["checkout", "master", "--quiet"])?;
    let needs_stash = git(&["diff", "--quiet", "--exit-code"]).is_err() || git(&["diff", "--cached", "--quiet", "--exit-code"]).is_err();
    if needs_stash {
        git(&["stash", "--quiet"])?;
    }

    // compute before state
    let beforemf = shipcat_filebacked::load_manifest(&svc, conf, region)?;
    let before = serde_yaml::to_string(&beforemf)?;

    // move git back
    if needs_stash {
        git(&["stash", "pop", "--quiet"])?;
    }
    git(&["checkout", "-", "--quiet"])?;

    // display diff
    shell_diff(&before, &after)
}

/// Fast local git compare of shipcat template
///
/// Because this uses the template in master against local state,
/// we don't resolve secrets for this (would compare equal values anyway).
pub fn template_vs_git(svc: &str, conf: &Config, region: &Region) -> Result<bool> {
    use crate::helm;
    let mock = true; // both would be equivalent vault reads anyway
    let afterpth = Path::new(".").join("after.shipcat.gen.yml");
    let _after = helm::direct::template(&svc, &region, &conf, None, mock, Some(afterpth.clone()))?;

    // move git to get before state:
    git(&["checkout", "master", "--quiet"])?;
    let needs_stash = git(&["diff", "--quiet", "--exit-code"]).is_err() || git(&["diff", "--cached", "--quiet", "--exit-code"]).is_err();
    if needs_stash {
        git(&["stash", "--quiet"])?;
    }

    // compute old state:
    let beforepth = Path::new(".").join("before.shipcat.gen.yml");
    let _before = helm::direct::template(&svc, &region, &conf, None, mock, Some(beforepth.clone()))?;

    // move git back
    if needs_stash {
        git(&["stash", "pop", "--quiet"])?;
    }
    git(&["checkout", "-", "--quiet"])?;

    // display diff
    // doesn't reuse shell_diff because we already have files from direct::template
    let args = ["-u", "before.shipcat.gen.yml", "after.shipcat.gen.yml"];
    debug!("diff {}", args.join(" "));
    let s = Command::new("diff").args(&args).status()?;
    // cleanup
    fs::remove_file(beforepth)?;
    fs::remove_file(afterpth)?;
    Ok(s.success())
}


use std::path::Path;
use std::fs::{self, File};
use std::io::Write;

/// Diff values using kubectl diff
///
/// Generate crd as we write it and pipe it to `kubectl diff -`
/// Only works on clusters with kubectl 1.13 on the server side, so not available everywhere
pub fn values_vs_kubectl(svc: &str, conf: &Config, region: &Region) -> Result<bool> {
    // Generate crd in a temp file:
    let mf = shipcat_filebacked::load_manifest(svc, conf, region)?;
    let crd = Crd::from(mf);
    let encoded = serde_yaml::to_string(&crd)?;
    let cfile = format!("{}.shipcat.crd.gen.yml", svc);
    let pth = Path::new(".").join(cfile);
    debug!("Writing crd for {} to {}", svc, pth.display());
    let mut f = File::create(&pth)?;
    writeln!(f, "{}", encoded)?;
    // shell out to kubectl:
    let (out, success) = kube::diff(pth.clone(), &region.namespace)?;
    println!("{}", out);
    // cleanup:
    fs::remove_file(pth)?;
    Ok(success)
}

/// Diff using template kubectl diff
///
/// Generate template as we write it and pipe it to `kubectl diff -`
/// Only works on clusters with kubectl 1.13 on the server side, so not available everywhere
pub fn template_vs_kubectl(svc: &str, conf: &Config, region: &Region, mock: bool) -> Result<bool> {
    use crate::helm;
    // Generate template in a temp file:
    let tfile = format!("{}.shipcat.tpl.gen.yml", svc);
    let pth = Path::new(".").join(tfile);
    let version = None; // TODO: override in rolling?
    helm::direct::template(&svc, &region, &conf, version, mock, Some(pth.clone()))?;

    let (out, success) = kube::diff(pth.clone(), &region.namespace)?;
    println!("{}", out);
    // cleanup:
    fs::remove_file(pth)?;
    Ok(success)
}

// Compare using diff(1)
// difference libraries all seemed to be lacking somewhat
fn shell_diff(before: &str, after: &str) -> Result<bool> {
    let beforepth = Path::new(".").join("before.shipcat.gen.yml");
    debug!("Writing before to {}", beforepth.display());
    let mut f = File::create(&beforepth)?;
    writeln!(f, "{}", before)?;

    let afterpth = Path::new(".").join("after.shipcat.gen.yml");
    debug!("Writing after to {}", afterpth.display());
    let mut f = File::create(&afterpth)?;
    writeln!(f, "{}", after)?;

    let args = ["-u", "before.shipcat.gen.yml", "after.shipcat.gen.yml"];
    debug!("diff {}", args.join(" "));
    let s = Command::new("diff").args(&args).status()?;
    // cleanup
    fs::remove_file(beforepth)?;
    fs::remove_file(afterpth)?;

    Ok(s.success())
}
