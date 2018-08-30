/// This file contains all the hidden `shipcat list-*` subcommands
use std::io::{self, Write};
use super::product::Product;
use super::{Result, Manifest, Config};

/// Print the supported regions
pub fn regions(conf: &Config) -> Result<()> {
    for (r, _) in &conf.regions {
        let _ = io::stdout().write(format!("{}\n", r).as_bytes());
    }
    Ok(())
}

/// Print the supported locations
pub fn locations(conf: &Config) -> Result<()> {
    for (r, _) in &conf.locations {
        let _ = io::stdout().write(format!("{}\n", r).as_bytes());
    }
    Ok(())
}

/// Print supported products in a location
pub fn products(conf: &Config, location: String) -> Result<()> {
    for product in Product::available()? {
        match Product::completed(&product, conf, &location) {
            Ok(p) => {
                if p.locations.contains(&location) {
                    let _ = io::stdout().write(&format!("{}\n", product).as_bytes());
                }
            }
            Err(e) => {
                bail!("Failed to examine product {}: {}", product, e)
            }
        }
    }
    Ok(())
}

/// Print supported services in a region
pub fn services(conf: &Config, region: String) -> Result<()> {
    let services = Manifest::available()?;
    // this call happens before kubectl config validation
    // make a best stab at context instead:
    let region_guess = conf.contextAliases.get(&region).unwrap_or(&region);
    // NB: we do this because shipcat autocomplete takes kube context
    // and pass it in here, so a kubectx for region is most likely!

    for svc in services {
        match Manifest::basic(&svc, conf, Some(region_guess.clone())) {
            Ok(mf) => {
                if mf.regions.contains(&region_guess) {
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
