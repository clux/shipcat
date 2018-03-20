/// This file contains all the hidden `shipcat list-*` subcommands
use std::io::{self, Write};
use super::{Result, Manifest, Config};

/// Print the supported regions
pub fn regions(conf: &Config) -> Result<()> {
    for (r, _) in &conf.regions {
        let _ = io::stdout().write(format!("{}\n", r).as_bytes());
    }
    Ok(())
}

pub fn services(conf: &Config, region: String) -> Result<()> {
    let services = Manifest::available()?;
    for svc in services {
        match Manifest::basic(&svc, conf, Some(region.clone())) {
            Ok(mf) => {
                if mf.regions.contains(&region) {
                    let _ = io::stdout().write(&format!("{}\n", svc).as_bytes());
                }
            }
            Err(e) => {
                bail!("Failed to examine manifest for {}: {}", svc, e)
            }
        }
    }
    Ok(())
}
