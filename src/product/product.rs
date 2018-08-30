use serde_yaml;
use regex::Regex;
use walkdir::WalkDir;

use std::io::prelude::*;
use std::fs::File;
use std::path::{PathBuf, Path};

use super::{Result, Config};

// structs re-used from Manifest:
use super::structs::Contact;

use manifest::Manifest;


// -- dependent structs used by Product (currently just inlined here)

/// Product owner
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Owner {
    /// Contact details (flattened into Owner)
    #[serde(flatten)]
    pub contact: Contact,
}
impl Owner {
    fn verify(&self, _: &Config) -> Result<()> {
        self.contact.verify()?;
        Ok(())
    }
}

/// Service depended on by Product
#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Service {
    /// Name of service relied upon (used to goto dependent manifest)
    pub name: String,
    /// Region that the service exists in
    pub region: String,
}
impl Service {
    fn verify(&self, config: &Config, location: &str) -> Result<()> {
        // self.name must exist in services/
        let dpth = Path::new(".").join("services").join(self.name.clone());
        if !dpth.is_dir() {
            bail!("Service {} does not exist in services/", self.name);
        }

        // specified region must be valid
        let manifest = Manifest::basic(&self.name, config, None)?;
        if self.region == "" {
            bail!("Service {} has no region specified", self.name);
        } else if !manifest.regions.contains(&self.region) {
            bail!("Service {} does not exist in region {}", self.name, self.region);
        }

        // region must service this location
        if !config.regions[&self.region].locations.contains(&location.to_string()) {
            bail!("Service {} uses region {}, which is not available in location {}",
                self.name, self.region, location);
        }

        Ok(())
    }
}

// -- end dependent structs


/// Product manifest, serializable from product.yml
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Product {
    /// Name of the product
    #[serde(default)]
    pub name: String,
    /// Description of the product
    #[serde(default)]
    pub description: String,

    /// Version of product
    ///
    /// Disjoint from service versions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Location injected by constructor
    #[serde(default, skip_deserializing)]
    pub location: String,

    /// Locations product is active in
    #[serde(default, skip_serializing)]
    pub locations: Vec<String>,

    /// Owner of the service
    #[serde(default)]
    pub owner: Option<Owner>,

    /// Jira ticket
    #[serde(default)]
    pub jira: Option<String>,

    /// Service dependencies
    #[serde(default)]
    pub services: Vec<Service>,
}

impl Product {
    /// Walk the products directory and return the available products
    pub fn available() -> Result<Vec<String>> {
        let pdcdir = Path::new(".").join("products");
        let pdcts = WalkDir::new(&pdcdir)
            .min_depth(1)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir());

        let mut xs = vec![];
        for e in pdcts {
            let mut cmps = e.path().components();
            cmps.next(); // .
            cmps.next(); // products
            let svccomp = cmps.next().unwrap();
            let svcname = svccomp.as_os_str().to_str().unwrap();
            xs.push(svcname.into());
        }
        Ok(xs)
    }

    fn basic(product: &str, conf: &Config) -> Result<Product> {
        let pth = Path::new(".").join("products").join(product);
        if !pth.exists() {
            bail!("Product folder {} does not exist", pth.display())
        }
        let mut p = Product::read_from(&pth)?;
        p.pre_merge_implicits(conf)?;
        p.post_merge_implicits(conf, None)?;
        Ok(p)
    }

    /// Read a Product file in an arbitrary path
    fn read_from(pwd: &PathBuf) -> Result<Product> {
        let mpath = pwd.join("product.yml");
        trace!("Using product manifest in {}", mpath.display());
        if !mpath.exists() {
            bail!("Product file {} does not exist", mpath.display())
        }
        let mut f = File::open(&mpath)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        Ok(serde_yaml::from_str(&data)?)
    }

    /// Fill in env overrides and apply merge rules
    pub fn fill(&mut self, conf: &Config, location: &str) -> Result<()> {
        self.pre_merge_implicits(conf)?;
        // merge location specific overrides if they exist
        let envlocals = Path::new(".")
            .join("products")
            .join(&self.name)
            .join(format!("{}.yml", location));
        if envlocals.is_file() {
            debug!("Merging location locals from {}", envlocals.display());
            self.merge(&envlocals)?;
        }
        self.post_merge_implicits(conf, Some(location.into()))?;
        Ok(())
    }

    /// Read and environment merged product manifest
    pub fn completed(service: &str, conf: &Config, location: &str) -> Result<Product> {
        let _r = &conf.locations[location]; // tested for existence earlier;
        let pth = Path::new(".").join("products").join(service);
        if !pth.exists() {
            bail!("Product folder {} does not exist", pth.display())
        }
        let mut px = Product::read_from(&pth)?;
        // fill defaults and merge locations
        px.fill(conf, location)?;
        Ok(px)
    }

    /// Verify assumptions about product
    ///
    /// Assumes the product has been populated with `implicits`
    pub fn verify(&self, conf: &Config) -> Result<()> {
        assert!(self.location != ""); // needs to have been set by implicits!
        // limit to 50 characters, alphanumeric, dashes for sanity.
        // 63 is kube dns limit (13 char suffix buffer)
        let re = Regex::new(r"^[0-9a-z\-]{1,50}$").unwrap();
        if !re.is_match(&self.name) {
            bail!("Please use a short, lower case product names with dashes");
        }
        if self.name.ends_with('-') || self.name.starts_with('-') {
            bail!("Please use dashes to separate words only");
        }


        if let Some(ref md) = self.owner {
            md.verify(&conf)?;
        } else {
            bail!("Missing owner for {}", self.name);
        }

        // vectorised entries
        for d in &self.services {
            d.verify(&conf, &self.location)?;
        }

        // version
        if self.version.is_none() {
            bail!("No version set");
        }

        // JIRA
        if let Some(ticket) = &self.jira {
            let pattern = r"^[A-Z]+-[0-9]+$";
            let re = Regex::new(&pattern).unwrap();
            if !re.is_match(&ticket) {
                bail!("Jira ticket '{}' does not match expected format: {}", ticket, pattern);
            }
        } else {
            bail!("No JIRA ticket set");
        }

        // locations
        for l in &self.locations {
            // 1) is it a valid location name?
            if conf.locations.get(l).is_none() {
                bail!("Unsupported location {} without entry in config", l);
            }
        }
        // 3) is the provided location listed in the main yaml?
        if !self.locations.contains(&self.location.to_string()) {
            bail!("Unsupported location {} for product {}", self.location, self.name);
        }

        Ok(())
    }
}

/// Entry point for product verify [--location location] products...
pub fn validate(products: Vec<String>, conf: &Config, location: Option<String>) -> Result<()> {
    for pname in products {

        let locations = if let Some(l) = &location {
            vec![l.to_string()]
        } else {
            Product::basic(&pname, conf)?.locations
        };
        for l in &locations {
            debug!("validating product {} for {}", pname, &l);
            Product::completed(&pname, conf, l)?.verify(conf)?;
            info!("validated product {} for {}", pname, &l);
        }
    }
    Ok(())
}

/// Entry point for product show [product] [location]
pub fn show(product: Option<String>, conf: &Config, location: &str) -> Result<()> {
    use std::io::{self, Write};
    let encoded = if let Some(pname) = product {
        let p = Product::completed(&pname, conf, location)?;
        serde_yaml::to_string(&p)?
    } else {
        let mut px = vec![];
        for pname in Product::available()? {
            let p = Product::completed(&pname, conf, location)?;
            px.push(p);
        }
        serde_yaml::to_string(&px)?
    };
    let _ = io::stdout().write(&format!("{}\n", encoded).as_bytes());
    Ok(())
}


#[cfg(test)]
mod tests {
    use tests::setup;
    use super::Config;
    use super::Product;

    #[test]
    fn product_test() {
        setup();
        let conf = Config::read().unwrap();
        let p = Product::completed("triage", &conf, "uk").unwrap();
        let res = p.verify(&conf);
        assert!(res.is_ok(), "verified product");
    }
}
