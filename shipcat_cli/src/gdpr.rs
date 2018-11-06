use serde_yaml;
use std::collections::BTreeMap;

use super::structs::security::DataHandling;
use super::{Manifest, Result, Config};

/// GdprOutput across manifests
#[derive(Serialize)]
struct GdprOutput {
    pub mappings: BTreeMap<String, DataHandling>,
    pub services: Vec<String>,
}


/// Show GDPR related info for a service
///
/// Prints the cascaded structs from a manifests `dataHandling`
pub fn show(svc: Option<String>, conf: &Config, region: &str) -> Result<()> {
    if let Some(s) = svc {
        let mf = Manifest::stubbed(&s, conf, region)?;
        let out = serde_yaml::to_string(&mf.dataHandling.unwrap_or_else(|| DataHandling::default()))?;
        println!("{}", out);
    } else {
        let mut mappings = BTreeMap::new();
        let mut services = vec![];
        for s in Manifest::available()? {
            let mf = Manifest::stubbed(&s, conf, region)?;
            // only include the entries that have it specified
            if let Some(dh) = mf.dataHandling {
                mappings.insert(s.clone(), dh);
            }
            services.push(s);
        }
        let data = GdprOutput { mappings, services };
        let out = serde_yaml::to_string(&data)?;
        println!("{}", out);
    }
    Ok(())
}
