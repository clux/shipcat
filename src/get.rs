/// This file contains the `shipcat get` subcommand
use std::collections::BTreeMap;
use semver::Version;
use serde_json;
use super::{Result, Manifest, Config};

pub fn versions(conf: &Config, region: &str) -> Result<()> {
    let services = Manifest::available()?;
    let mut output : BTreeMap<String, Version> = BTreeMap::new();

    for svc in services {
        let mf = Manifest::stubbed(&svc, &conf, &region)?;
        if mf.regions.contains(&region.to_string()) {
            if let Some(v) = mf.version {
                if let Ok(sv) = Version::parse(&v) {
                    output.insert(svc, sv);
                }
            }
        }
    }
    let _ = println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

pub fn images(conf: &Config, region: &str) -> Result<()> {
    let services = Manifest::available()?;
    let mut output : BTreeMap<String, String> = BTreeMap::new();

    for svc in services {
        let mf = Manifest::stubbed(&svc, &conf, &region)?;
        if mf.regions.contains(&region.to_string()) {
            if let Some(i) = mf.image {
                output.insert(svc, i);
            }
        }
    }
    let _ = println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
