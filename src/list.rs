/// This file contains all the hidden `shipcat list-*` subcommands
use std::io::{self, Write};
use super::{Result, Manifest};

/// Print the supported regions
pub fn regions() -> Result<()> {
    // TODO: look for override files in the environments folder!
    let _ = io::stdout().write(b"dev-uk\n");
    let _ = io::stdout().write(b"dev-global1\n");
    let _ = io::stdout().write(b"dev-ops\n");
    Ok(())
}

pub fn services(region: String) -> Result<()> {
    let services = Manifest::available()?;
    for svc in services {
        match Manifest::basic(&svc) {
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
