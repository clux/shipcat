/// This file contains all the hidden `shipcat list-*` subcommands
use super::{Result, Region, Manifest, Config}; //Product

/// Print the supported regions
pub fn regions(conf: &Config) -> Result<()> {
    for r in &conf.list_regions() {
        println!("{}", r);
    }
    Ok(())
}

/// Print the supported locations
pub fn locations(conf: &Config) -> Result<()> {
    for (r, _) in &conf.locations {
        println!("{}", r);
    }
    Ok(())
}

/// Print supported products in a location
//pub fn products(conf: &Config, location: String) -> Result<()> {
//    for product in Product::available()? {
//        match Product::completed(&product, conf, &location) {
//            Ok(p) => {
//                if p.locations.contains(&location) {
//                    println!("{}", product);
//                }
//            }
//            Err(e) => {
//                bail!("Failed to examine product {}: {}", product, e)
//            }
//        }
//    }
//    Ok(())
//}

/// Print supported services in a region
/// TODO: this one needs to do the guess outside in main!
pub fn services(region: &Region) -> Result<()> {
    let services = Manifest::available(&region.name)?;
    for svc in services {
        println!("{}", svc);
    }
    Ok(())
}
