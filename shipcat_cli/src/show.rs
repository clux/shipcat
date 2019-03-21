use super::{Result, Config, Region};

/// Print the config
///
/// This allows debugging the config type after filtering/completing
pub fn config(conf: Config) -> Result<()> {
    conf.print()?;
    Ok(())
}

use shipcat_definitions::Crd;
pub fn config_crd(conf: Config) -> Result<()> {
    if conf.has_all_regions() {
        bail!("The config crd needs to be for a single region only");
    }
    let crd = Crd::from(conf);
    println!("{}", serde_yaml::to_string(&crd)?);
    Ok(())
}

pub fn manifest_crd(svc: &str, conf: &Config, reg: &Region) -> Result<()> {
    let mf = shipcat_filebacked::load_manifest(svc, conf, reg)?;
    let crd = Crd::from(mf);
    println!("{}", serde_yaml::to_string(&crd)?);
    Ok(())
}
