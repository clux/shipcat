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

fn generate_kong_output(conf: &Config, region: &str) -> Result<KongOutput> {
    let mut apis = BTreeMap::new();

    // Generate list of APIs to feed to Kong
    for svc in Manifest::available()? {
        debug!("Scanning service {:?}", svc);
        let mf = Manifest::stubbed(&svc, conf, region)?; // does not need secrets
        if !mf.disabled && mf.regions.contains(&region.to_string()) {
            debug!("Found service {} in region {}", mf.name, region);
            if let Some(k) = mf.kong {
                apis.insert(svc, k);
            }
        }
    }

    // Add general Kong region config
    let reg = conf.regions[region].clone();
    for (name, api) in reg.kong.extra_apis.clone() {
        apis.insert(name, api);
    }
    Ok(KongOutput { apis, kong: reg.kong })
}

/// Generate Kong config
///
/// Generate a JSON file used to configure Kong for a given region
pub fn kong_generate(conf: &Config, region: &str) -> Result<()> {
    let output = generate_kong_output(conf, &region)?;
    let _ = io::stdout().write(serde_json::to_string_pretty(&output)?.as_bytes());
    Ok(())
}

/// Return the config_url for the given region
pub fn kong_config_url(conf: &Config, region: &str) -> Result<()> {
    let reg = conf.regions[&region.to_string()].clone();
    println!("{}", reg.kong.config_url);
    Ok(())
}

pub fn reconcile(conf: &Config, region: &str) -> Result<()> {
    use std::env;
    use std::path::Path;
    use std::fs::File;
    use std::io::{Write};
    let reg = conf.regions[&region.to_string()].clone();

    let kong = generate_kong_output(conf, region)?;
    let output = serde_json::to_string_pretty(&kong)?;

    // write kong-{region}.json
    let fname = format!("kong-{}.json", region);
    let pth = Path::new(".").join(&fname);
    debug!("Writing kong values for {} to {}", region, pth.display());
    let mut f = File::create(&pth)?;
    write!(f, "{}\n", output)?;
    debug!("Wrote kong values for {} to {}: \n{}", region, pth.display(), output);

    // guess kong-configurator location
    let kongpth = format!("{}/kong.py",
        env::var("KONG_CONFIGURATOR_DIR").unwrap_or("/kong-configurator".into())
    );
    // python3 /kong-configurator/kong.py -c kong-{region}.json -u $KONG_URL
    use std::process::Command;
    let args = vec![kongpth, "-c".into(), fname, "-u".into(), reg.kong.config_url];
    info!("python3 {}", args.join(" "));
    let s = Command::new("python3").args(&args).status()?;
    if !s.success() {
        bail!("Subprocess failure from kong configurator: {}", s.code().unwrap_or(1001))
    }
    Ok(())
}
