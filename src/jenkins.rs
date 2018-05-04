use super::{Result, ErrorKind};
use chrono::{Utc, TimeZone};
use jenkins_api::{JenkinsBuilder, Jenkins, Build, Job};
//use jenkins_api::{Parameter, StringParameterValue};
use jenkins_api::action::*;
use std::env;
use std::collections::BTreeMap;
use std::io::{self, Write};

fn env_user() -> Result<String> {
    env::var("JENKINS_API_USER").map_err(|_| ErrorKind::MissingJenkinsUser.into())
}
fn env_pass() -> Option<String> {
    env::var("JENKINS_API_TOKEN").ok()
}

fn env_url() -> Result<String> {
    env::var("JENKINS_URL").map_err(|_| ErrorKind::MissingJenkinsUrl.into())
}

fn get_client() -> Result<Jenkins> {
    Ok(JenkinsBuilder::new(&env_url()?)
        .with_user(&env_user()?, env_pass().as_ref().map(String::as_str))
        .build().map_err(|e| {
            error!("Failed to create jenkins client {}", e);
            ErrorKind::JenkinsFailure
        })?
    )
}

fn get_job(client: &Jenkins, job: &str) -> Result<Job> {
    Ok(client.get_job(job).map_err(|e| {
        error!("Failed to get job {}", e);
        ErrorKind::MissingJenkinsJob(job.into())
    })?)
}

pub fn get_string_params(b: &Build) -> BTreeMap<String, String> {
    let mut res = BTreeMap::new();
    for a in &b.actions {
        if let &Action::ParametersAction { ref parameters } = a {
            trace!("got pars {:?}", parameters);
            for p in parameters {
                if let &Parameter::StringParameterValue { ref name, ref value } = p {
                    res.insert(name.clone(), value.clone());
                }
            }
        }
    }
    res
}

fn find_build_by_parameter(client: &Jenkins, job: &str, app: &str) -> Result<Option<Build>> {
    let job = get_job(&client, job)?;
    let len = job.builds.len();
    for sbuild in job.builds {
        match sbuild.get_full_build(&client) {
            Ok(build) => {
                debug!("scanning build :{:?}", build);
                let params = get_string_params(&build);
                if let Some(a) = params.get("APP") {
                    if a == app {
                        return Ok(Some(build));
                    }
                }
            }
            Err(_) => continue,
        }
    }
    warn!("No completed deploy jobs found for {} in the last {} builds", app, len);
    Ok(None)
}

fn find_builds_by_parameter(client: &Jenkins, job: &str, app: &str) -> Result<Vec<Build>> {
    let job = get_job(&client, job)?;
    let mut builds = vec![];
    let len = job.builds.len();
    for sbuild in job.builds {
        match sbuild.get_full_build(&client) {
            Ok(build) => {
                debug!("scanning build :{:?}", build);
                let params = get_string_params(&build);
                if let Some(a) = params.get("APP") {
                    if a == app {
                        builds.push(build);
                    }
                }
            }
            Err(_) => continue,
        }
    }
    if builds.is_empty() {
        warn!("No completed deploy jobs found for {} in the last {} builds", app, len);
    }
    Ok(builds)
}

fn find_build_by_nr(client: &Jenkins, job: &str, nr: u32, app: &str) -> Result<Option<Build>> {
    let job = get_job(&client, job)?;
    let len = job.builds.len();
    for sbuild in job.builds {
        if sbuild.number == nr {
            match sbuild.get_full_build(&client) {
                Ok(build) => {
                    let params = get_string_params(&build);
                    if let Some(a) = params.get("APP") {
                        if a == app {
                            return Ok(Some(build))
                        }
                        else {
                            warn!("Build found, but it's not for {}", app);
                            return Ok(None)
                        }
                    }
                },
                Err(_) => {
                    warn!("Failed to fetch build number {}", app);
                    return Ok(None)
                }
            }
        }
    }
    warn!("Build number {} not found for {} in last {} builds", nr, app, len);
    Ok(None)
}

/// Print the latest deployment job for a service in a given region
pub fn latest_build(svc: &str, reg: &str) -> Result<()> {
    let client = get_client()?;
    let jobname = format!("kube-deploy-{}", reg);
    if let Some(build) = find_build_by_parameter(&client, &jobname, svc)? {
        let ts = Utc.timestamp((build.timestamp/1000) as i64, 0);
        println!("{}#{} ({}) at {} on {}",
            jobname, build.number, build.queue_id, ts, build.url
        );
    }
    Ok(())
}

/// Print a history for the last deployment jobs in the given region
///
/// Analogue to `helm history {service}` but queries jenkins directly.
pub fn history(service: &str, reg: &str) -> Result<()> {
    let client = get_client()?;
    let jobname = format!("kube-deploy-{}", reg);
    let builds = find_builds_by_parameter(&client, &jobname, service)?;

    if builds.is_empty() {
        return Ok(())
    }
    println!("{0:<6} {1:<20} {2:<9}", "BUILD", "UPDATED", "RESULT");
    for b in builds {
        let ts = Utc.timestamp((b.timestamp/1000) as i64, 0);
        let stamp = ts.format("%Y-%m-%d %H:%M:%S").to_string();
        let link = format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", b.url, b.number);
        // not aligning the build because it's full of escape codes for the link
        println!("{0}   {1:<20} {2:<9?}", link, stamp, b.result);
    }
    Ok(())
}

/// Print the consoleText from the latest deployment job for a service in a give region
pub fn latest_console(svc: &str, reg: &str) -> Result<()> {
    let client = get_client()?;
    let jobname = format!("kube-deploy-{}", reg);
    if let Some(build) = find_build_by_parameter(&client, &jobname, svc)? {
        let console = build.get_console(&client).unwrap();
        let _ = io::stdout().write(&console.as_bytes());
    }
    Ok(())
}

/// Print the consoleText from a specific deployment nr for a service in a give region
pub fn specific_console(svc: &str, nr: u32, reg: &str) -> Result<()> {
    let client = get_client()?;
    let jobname = format!("kube-deploy-{}", reg);
    if let Some(build) = find_build_by_nr(&client, &jobname, nr, svc)? {
        let console = build.get_console(&client).unwrap();
        // allow piping this
        let _ = io::stdout().write(&console.as_bytes());
    }
    Ok(())
}
