use super::{Result, Manifest};
use shipcat_definitions::Crd;
use serde::Serialize;
use regex::Regex;
use serde_yaml;
use chrono::{Utc, DateTime};

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
    let err : String = String::from_utf8_lossy(&s.stderr).into();
    if !err.is_empty() {
        warn!("kubectl {} stderr: {}", args.join(" "), err.trim());
    }
    // kubectl keeps returning opening and closing apostrophes - strip them:
    if out.len() > 2 && out.starts_with('\'') {
        let res = out.split('\'').collect::<Vec<_>>()[1];
        return Ok(res.trim().into());
    }
    Ok(out)
}

/// CLI way to resolve kube context
///
/// Should only be used from main.
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

fn rollout_status(mf: &Manifest) -> Result<bool> {
    // TODO: handle more than one deployment
    // Even if this were called 10 times with 1/10th of waiting time, we still can't wait:
    // - we'd still need to check other deploys...
    // - we'd have to deal with `kubectl rollout status` not having a timeout flag..
    let statusvec = vec![
        "rollout".into(),
        "status".into(),
        format!("deployment/{}", mf.name.clone()), // always one deployment with same name
        format!("-n={}", mf.namespace),
        "--watch=false".into(), // always just print current status
    ];
    let rollres = kout(statusvec)?;
    debug!("{}", rollres);
    if rollres.contains("successfully rolled out") {
        Ok(true)
    } else {
        // TODO: check if any of the new pods have restarts in them
        // will avoid waiting for the full time
        Ok(false)
    }
}

/// A replacement for helm upgrade's --wait and --timeout
pub fn await_rollout_status(mf: &Manifest) -> Result<bool> {
    use std::{thread, time};
    // Check for rollout progress
    let waittime = mf.estimate_wait_time();
    let sec = time::Duration::from_millis(1000);
    // if this is called immediately after apply/upgrade, resources might not exist yet
    match rollout_status(&mf) {
        Ok(true) => return Ok(true), // can also insta-succeed on "noops"
        Ok(false) => debug!("Ignoring rollout failure right after upgrade"),
        Err(e) => warn!("Ignoring rollout failure right after upgrade: {}", e),
    };
    info!("Waiting {}s for deployment {} to rollout (not ready yet)", waittime, mf.name);
    for i in 1..10 {
        trace!("poll iteration {}", i);
        let mut waited = 0;
        // sleep until 1/10th of estimated upgrade time and poll for status
        while waited < waittime/10 {
            waited += 1;
            trace!("sleep 1s (waited {})", waited);
            thread::sleep(sec);
        }
        if rollout_status(&mf)? {
            return Ok(true)
        }
    }
    Ok(false) // timeout
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

/// Return non-running or partially ready pods
fn get_broken_pods(mf: &Manifest) -> Result<(String, Vec<String>)> {
    let podargs = vec![
        "get".into(),
        "pods".into(),
        format!("-l=app={}", mf.name),
        format!("-n={}", mf.namespace),
        "--no-headers".into(),
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
        else if let Some(caps) = status_re.captures(l) {
            if caps["ready"] != caps["total"] {
                if let Some(p) = l.split(' ').next() {
                    warn!("Found pod with less than necessary containers healthy: {}", p);
                    bpods.push(p.into());
                }
            }
        }
    }
    Ok((podres, bpods))
}

/// Debug helper when upgrades fail
///
/// Prints log excerpts and events for broken pods.
/// Typically enough to figure out why upgrades broke.
pub fn debug(mf: &Manifest) -> Result<()> {
    let (podres, pods) = get_broken_pods(&mf)?;
    if pods.is_empty() {
        info!("No broken pods found");
        info!("Pod statuses:\n{}", podres);
    } else {
        warn!("Pod statuses:\n{}", podres);
    }
    for pod in pods.clone() {
        warn!("Debugging non-running pod {}", pod);
        let logvec = vec![
            "logs".into(),
            pod.clone(),
            mf.name.clone(),
            format!("-n={}", mf.namespace),
            "--tail=30".into(),
        ];
        match kout(logvec) {
            Ok(l) => {
                if l == "" {
                    warn!("No logs for pod {} found", pod);
                } else {
                    warn!("Last 30 log lines:");
                    println!("{}", l);
                }
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
                    println!("{}", o.split_off(idx))
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
    // ignore errors from here atm - it's mostly here as a best effort helper
    let _ = debug_active_replicasets(mf);
    Ok(())
}


// Parsed replica status from a deployment description
#[derive(Clone, Debug)]
struct ReplicaStatus {
    /// Name of ReplicaSet
    pub name: String,
    /// Total replicas expected
    pub total: u32,
    /// Total replicas running
    pub running: u32,
}

// limited parsed struct from kube ReplicaSet yaml
#[derive(Deserialize)]
struct ReplicaSetVal {
    // rollout status of the RS
    status: ReplicaInfoVal,
    // metadata for timestamps
    metadata: ReplicaMetadataVal,
}
#[derive(Deserialize, Debug)]
struct ReplicaMetadataVal {
    creationTimestamp: DateTime<Utc>, // e.g. 2018-05-17T10:08:37Z
}
#[derive(Deserialize, Debug)]
struct ReplicaInfoVal {
    // `observedGeneration` <- not unique across replicasets so useless
    // Currently available replicas (ready for minReadySeconds) from this generation
    #[serde(default)] // 0 if missing
    availableReplicas: u32,
    /// Currently ready replilas from this generation (weaker than above)
    #[serde(default)] // 0 if missing
    readyReplicas: u32,
    /// Most recently oberved number of replicas
    replicas: u32,
}


/// Simplified ReplicaSet struct
///
/// Created by combining deployment description with appropriate replicaset yamls
#[derive(Debug)]
pub struct ReplicaSet {
    /// Name of replicaset
    pub name: String,
    /// Available replicas (has been ready for minReadySeconds)
    pub available: u32,
    /// Total replicas in set
    pub total: u32,
    /// Created timestamp (used for ordering)
    pub created: DateTime<Utc>,
}

// Finds affected replicatsets, and checks the state of them.
fn find_active_replicasets(mf: &Manifest) -> Result<Vec<ReplicaSet>> {
    // find relevant replicasets
    // NB: not returned via `k get deploy {name} -oyaml` - have to scrape describe..
    let descvec = vec![
        "describe".into(),
        "deploy".into(),
        mf.name.clone(),
        format!("-n={}", mf.namespace),
    ];
    // Finding the affected replicasets:
    let rs_re = Regex::new(r"(Old|New)ReplicaSets?:\s+(?P<rs>\S+)\s+\((?P<running>\d+)/(?P<total>\d+)").unwrap();
    let deployres = kout(descvec)?;
    let mut sets = vec![];
    for l in deployres.lines() {
        if let Some(caps) = rs_re.captures(l) {
            sets.push(ReplicaStatus {
                name: caps["rs"].to_string(),
                running: caps["running"].parse()?,
                total: caps["total"].parse()?,
            });
        }
    }
    // Query each of the affected replicasets for info using yaml api:
    let mut completesets = vec![];
    for rs in sets {
        // Find total counts from each replicaset
        let getvec = vec![
            "get".into(),
            "rs".into(),
            rs.name.clone(),
            format!("-n={}", mf.namespace),
            "-oyaml".into(),
        ];
        let getres = kout(getvec)?;
        let rv : ReplicaSetVal = serde_yaml::from_str(&getres)?;
        let ri = rv.status;
        let res = ReplicaSet {
            name: rs.name,
            available: ri.availableReplicas,
            total: ri.replicas,
            created: rv.metadata.creationTimestamp
        };
        completesets.push(res);
    }
    Ok(completesets)
}

// Debug status of active replicasets post-upgrade helpful info
fn debug_active_replicasets(mf: &Manifest) -> Result<()> {
    let sets = find_active_replicasets(mf)?;
    if sets.len() > 1 {
        warn!("ReplicaSets: {:?}", sets);
    }
    if let Some(latest) = sets.iter().max_by_key(|x| x.created.timestamp()) {
        info!("Latest {:?}", latest);
        if latest.available > 0 && latest.available < latest.total {
            warn!("Some replicas successfully rolled out - maybe a higher timeout would help?");
        }
        else if latest.available == 0{
            warn!("No replicas were rolled out fast enough ({} secs)", mf.estimate_wait_time());
            warn!("Your application might be crashing, or fail to respond to healthchecks in time");
            warn!("Current health check is set to {:?}", mf.health);
        }
    } else {
        warn!("No active replicasets found");
    }
    Ok(())
}

/// Print upgrade status of current replicaset rollout
pub fn debug_rollout_status(mf: &Manifest) -> Result<()> {
    let mut sets = find_active_replicasets(mf)?;
    if sets.len() == 2 {
        sets.sort_unstable_by(|x,y| x.created.timestamp().cmp(&y.created.timestamp()));
        let old = sets.first().unwrap();
        let new = sets.last().unwrap();
        info!("{} upgrade status: old {}/{} -  new {}/{} ", mf.name,
            old.available, old.total,
            new.available, new.total
        );
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
///
/// Useful because we have autocomplete on manifest names in shipcat
pub fn port_forward(mf: &Manifest) -> Result<()> {
    // TODO: kubectl auth can-i create something?
    let port = mf.httpPort.unwrap();
    // first 1024 ports need sudo so avoid that
    let localport = if port <= 1024 { 7777 } else { port };

    debug!("Port forwarding kube deployment {} to localhost:{}", mf.name, localport);
    //kubectl port-forward deployment/${name} localport:httpPort
    let pfargs = vec![
        format!("-n={}", mf.namespace),
        "port-forward".into(),
        format!("deployment/{}", mf.name),
        format!("{}:{}", port, port)
    ];
    kexec(pfargs)?;
    Ok(())
}


/// Apply the CRD for any struct that can be turned into a CRD
///
/// CRDs itself, Manifest and Config typically.
pub fn apply_crd<T: Into<Crd<T>> + Serialize>(name: &str, data: T, ns: &str) -> Result<()> {
    use std::path::Path;
    use std::fs::{self, File};
    use std::io::Write;
    // Use trait constraint to convert it to a CRD
    let crd : Crd<T> = data.into();

    // Write it to a temporary file:
    let crdfile = format!("{}.crd.gen.yml", name);
    let pth = Path::new(".").join(&crdfile);
    debug!("Writing {} CRD for {} to {}", crd.kind, name, pth.display());
    let mut f = File::create(&pth)?;
    let encoded = serde_yaml::to_string(&crd)?;
    writeln!(f, "{}", encoded)?;
    debug!("Wrote {} CRD for {} to {}: \n{}", crd.kind, name, pth.display(), encoded);

    // Apply it using kubectl apply
    debug!("Applying {} CRD for {}", crd.kind, name);
    let applyargs = vec![
        format!("-n={}", ns),
        "apply".into(),
        "-f".into(),
        crdfile.clone(),
    ];
    debug!("applying {} : {:?}", name, applyargs);
    kexec(applyargs)?;
    let _ = fs::remove_file(&crdfile); // try to remove temporary file
    Ok(())
}
/// Find all ManifestCrds in a given namespace
///
/// Allows us to purge manifests that are not in Manifest::available()
fn find_all_manifest_crds(ns: &str) -> Result<Vec<String>> {
     let getargs = vec![
        "get".into(),
        format!("-n={}", ns),
        "shipcatmanifests".into(),
        "-ojsonpath='{.items[*].metadata.name}'".into(),
    ];
    let out = kout(getargs)?;
    if out == "''" { // stupid kubectl
        return Ok(vec![])
    }
    Ok(out.split(' ').map(String::from).collect())
}

use std::path::PathBuf;
// Kubectl diff experiment (ignores secrets)
pub fn diff(pth: PathBuf, ns: &str) -> Result<(String, bool)> {
    let args = vec![
        "diff".into(),
        format!("-n={}", ns),
        format!("-f={}", pth.display())
    ];
    // need the error code here so re-implent - and discard stderr
    use std::process::Command;
    debug!("kubectl {}", args.join(" "));

    let s = Command::new("kubectl").args(&args).output()?;
    let out : String = String::from_utf8_lossy(&s.stdout).into();
    let err : String = String::from_utf8_lossy(&s.stderr).into();
    trace!("out: {}, err: {}", out, err);
    if err.contains("the dryRun alpha feature is disabled") {
        bail!("kubectl diff is not supported in your cluster: {}", err.trim());
    }
    Ok((out, s.status.success()))
}

use std::collections::HashSet;
pub fn remove_redundant_manifests(ns: &str, svcs: &Vec<String>) -> Result<Vec<String>> {
    let requested: HashSet<_> = svcs.iter().cloned().collect();
    let found: HashSet<_> = find_all_manifest_crds(ns)?.iter().cloned().collect();
    debug!("Found manifests: {:?}", found);

    let excess : HashSet<_> = found.difference(&requested).collect();
    info!("Will remove excess manifests: {:?}", excess);
     let mut delargs = vec![
        "delete".into(),
        format!("-n={}", ns),
        "shipcatmanifests".into(),
    ];
    for x in &excess {
        delargs.push(x.to_string());
    }
    if !excess.is_empty() {
        let _res = kexec(delargs)?;
    } else {
        debug!("No excess manifests found");
    }
    let exvec = excess.into_iter().cloned().collect();
    Ok(exvec)
}

#[cfg(test)]
mod tests {
    use dirs;
    use super::current_context;

    #[test]
    fn validate_ctx() {
        let kubecfg = dirs::home_dir().unwrap().join(".kube").join("config");
        // ignoring this test on circleci..
        if kubecfg.is_file() {
            let ctx = current_context().unwrap();
            assert_eq!(ctx, ctx.trim());
            assert_ne!(ctx, "");
        }
    }
}
