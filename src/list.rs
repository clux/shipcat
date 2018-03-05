/// This file contains all the hidden `shipcat list-*` subcommands

use super::{Result, Manifest};

/// Print the supported regions
pub fn regions() -> Result<()> {
    // TODO: look for override files in the environments folder!
    println!("dev-uk");
    println!("dev-global1");
    println!("dev-ops");
    Ok(())
}

pub fn services(region: String) -> Result<()> {
    let services = Manifest::available()?;
    for svc in services {
        // Don't error handle heavily in here - used for autocomplete
        match Manifest::basic(&svc) {
            Ok(mf) => {
                if mf.regions.contains(&region) {
                    println!("{}", svc);
                }
            }
            Err(e) => warn!("Failed to examine manifest for {}: {}", svc, e)
        }
    }
    Ok(())
}
