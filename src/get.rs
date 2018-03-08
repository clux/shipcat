#![allow(non_snake_case)]
/// This file contains the `shipcat get` subcommand
use std::io::{self, Write};
use super::{Result, Manifest};


#[derive(Debug)]
pub enum ResourceType {
    VERSION,
    IMAGE,
}

pub fn table(rsrc: &str, quiet: bool, region: String) -> Result<()> {
    let resource = match rsrc {
        "version"|"ver" => ResourceType::VERSION,
        "image" => ResourceType::IMAGE,
        _ => {
            warn!("Supported resource types are: version, image");
            bail!("Unsupported resource {}", rsrc)
        }
    };

    let services = Manifest::available()?;
    if !quiet {
        println!("{0: <40} {1:?}", "NAME", resource);
    }
    for svc in services {
        let mf = Manifest::completed(&region, &svc, None)?;
        if mf.regions.contains(&region) {
            match resource {
                ResourceType::VERSION => {
                    println!("{0: <40} {1}", mf.name, mf.version.unwrap());
                },
                ResourceType::IMAGE => {
                    let img = format!("{}", mf.image.unwrap());
                    println!("{0: <40} {1}", mf.name, img);
                },
            }
        }
    }
    io::stdout().flush()?; // allow piping stdout elsewhere
    Ok(())
}
