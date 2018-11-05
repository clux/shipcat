use super::{Manifest, Config, Backend, Region};
use std::collections::BTreeMap;

use super::structs::security::DataHandling;
use super::{Result};

/// GdprOutput across manifests
#[derive(Serialize)]
struct GdprOutput {
    pub mappings: BTreeMap<String, DataHandling>,
    pub services: Vec<String>,
}


/// Show GDPR related info for a service
///
/// Prints the cascaded structs from a manifests `dataHandling`
pub fn show(svc: Option<String>, conf: &Config, region: &Region) -> Result<()> {
    let out = if let Some(s) = svc {
        let mf = Manifest::base(&s, conf, region)?;
        let data = if let Some(mut dh) = mf.dataHandling {
                dh
        } else {
            DataHandling::default()
        };
        serde_yaml::to_string(&data)?
    } else {
        let mut mappings = BTreeMap::new();
        let mut services = vec![];
        for s in Manifest::available(&region.name)? {
            let mf = Manifest::base(&s, conf, region)?;
            if let Some(mut dh) = mf.dataHandling {
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
