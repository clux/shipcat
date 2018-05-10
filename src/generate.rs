use std::path::Path;
use std::fs::File;
use std::io::prelude::*;
use std::io;

use serde_yaml;

use super::{Result};
use super::manifest::*;

/// Helm values -> stdout wrapper
///
/// Assumes you have called `Manifest::inline_configs` on a `completed` manifest.
pub fn values_stdout(mf: &Manifest) -> Result<()> {
    let encoded = serde_yaml::to_string(&mf)?;
    let _ = io::stdout().write(format!("{}\n", encoded).as_bytes());
    Ok(())
}

/// Helm values -> file wrapper
///
/// Assumes you have called `Manifest::inline_configs` on a `completed` manifest.
pub fn values_to_disk(mf: &Manifest, output: &str) -> Result<()> {
    let encoded = serde_yaml::to_string(&mf)?;

    let pth = Path::new(".").join(output);
    info!("Writing helm values for {} to {}", mf.name, pth.display());
    let mut f = File::create(&pth)?;
    write!(f, "{}\n", encoded)?;
    debug!("Wrote helm values for {} to {}: \n{}", mf.name, pth.display(), encoded);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::Manifest;
    use super::values_stdout;
    use tests::setup;
    use super::super::Config;

    #[test]
    fn helm_create() {
        setup();
        let conf = Config::read().unwrap();
        let mf = Manifest::stubbed("fake-ask", &conf, "dev-uk".into()).unwrap();
        if let Err(e) = values_stdout(&mf) {
            println!("Failed to create helm values for fake-ask");
            print!("{}", e);
            assert!(false);
        }
        // can verify output here matches what we want if we wanted to,
        // but type safety proves 99% of that anyway
    }
}
