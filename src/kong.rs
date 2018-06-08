use serde_json;
use std::io::{self, Write};
use std::collections::BTreeMap;

use super::{Manifest, Result, Config};
use super::structs::Kong;
use super::config::KongConfig;

/// KongOutput matches the format expected by the Kong Configurator script
#[derive(Serialize)]
struct KongOutput {
    pub apis: BTreeMap<String, Kong>,
    pub kong: KongConfig,
}

/// Generate Kong config
///
/// Generate a JSON file used to configure Kong for a given region
pub fn kong_generate(conf: &Config, region: String) -> Result<()> {
    let mut apis = BTreeMap::new();

    // Generate list of APIs to feed to Kong
    for svc in Manifest::available()? {
        debug!("Scanning service {:?}", svc);
        let mf = Manifest::stubbed(&svc, conf, &region)?; // does not need secrets
        if !mf.disabled && mf.regions.contains(&region) {
            debug!("Found service {} in region {}", mf.name, region);
            if let Some(k) = mf.kong {
                apis.insert(svc, k);
            }
        }
    }

    // Add general Kong region config
    let reg = conf.regions[&region].clone();
    for (name, api) in reg.kong.extra_apis.clone() {
        apis.insert(name, api);
    }
    let output = KongOutput { apis, kong: reg.kong };
    let _ = io::stdout().write(serde_json::to_string(&output)?.as_bytes());

    Ok(())
}

/// Return the config_url for the given region
pub fn kong_config_url(conf: &Config, region: String) -> Result<()> {
    let reg = conf.regions[&region].clone();
    println!("{}", reg.kong.config_url);
    Ok(())
}
