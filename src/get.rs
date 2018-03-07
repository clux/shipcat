#![allow(non_snake_case)]
/// This file contains the `shipcat get` subcommand
use std::io::{self, Write};
use super::{Result, Manifest};


#[derive(Debug)]
pub enum ResourceType {
    VERSION,
}

pub fn table(rsrc: &str, quiet: bool, region: String) -> Result<()> {
    let resource = match rsrc {
        "version"|"ver" => ResourceType::VERSION,
        _ => {
            warn!("Supported resource types are: version");
            bail!("Unsupported resource {}", rsrc)
        }
    };

    let services = Manifest::available()?;
    if !quiet {
        println!("{0: <30} {1:?}", "NAME", resource);
    }
    for svc in services {
        let mf = Manifest::completed(&region, &svc, None)?;
        if mf.regions.contains(&region) {
            match &resource {
                _VERSION => {
                    println!("{0: <30} {1}", mf.name, mf.image.unwrap().tag.unwrap());
                }
            }
        }
    }
    io::stdout().flush()?; // allow piping stdout elsewhere
    Ok(())
}
