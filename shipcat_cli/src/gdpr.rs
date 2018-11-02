use shipcat_definitions::config::{Region, ManifestDefaults};
use shipcat_definitions::Backend;
use std::collections::BTreeMap;

use super::structs::security::DataHandling;
use super::{Manifest, Result};

/// GdprOutput across manifests
#[derive(Serialize)]
struct GdprOutput {
    pub mappings: BTreeMap<String, DataHandling>,
    pub services: Vec<String>,
}


/// Show GDPR related info for a service
///
/// Prints the cascaded structs from a manifests `dataHandling`
pub fn show(svc: Option<String>, defs: &ManifestDefaults, region: &Region) -> Result<()> {
    let out = if let Some(s) = svc {
        let mf = Manifest::raw(&s, region)?;
        serde_yaml::to_string(&mf.dataHandling.unwrap_or_else(|| DataHandling::default()))?
    } else {
        let mut mappings = BTreeMap::new();
        let mut services = vec![];
        for s in Manifest::available(&region.name)? {
            // NB: this needs stubbed over raw because it needs dathandling implicits!
            let mf = Manifest::stubbed(&s, defs, region)?;
            // only include the entries that have it specified
            if let Some(dh) = mf.dataHandling {
                mappings.insert(s.clone(), dh);
            }
            services.push(s);
        }
        let data = GdprOutput { mappings, services };
        serde_yaml::to_string(&data)?
    };
    println!("{}", out);
    Ok(())
}
