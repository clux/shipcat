/// This file contains all the hidden `shipcat list-*` subcommands
use super::{Config, Region, Result};

/// Print the supported regions
pub fn regions(conf: &Config) -> Result<()> {
    for r in conf.list_regions() {
        println!("{}", r);
    }
    Ok(())
}

/// Print the supported locations
pub fn locations(conf: &Config) -> Result<()> {
    for r in conf.locations.keys() {
        println!("{}", r);
    }
    Ok(())
}

/// Print supported services in a region
/// TODO: this one needs to do the guess outside in main!
pub async fn services(conf: &Config, region: &Region) -> Result<()> {
    let services = shipcat_filebacked::available(conf, region).await?;
    for svc in services {
        println!("{}", &svc.base.name);
    }
    Ok(())
}
