use super::{parallel, UpgradeMode};
use super::{Result, Config, Manifest, ErrorKind};

/// Helm upgrade a service and its dependent components in parallel
pub fn upgrade(mf: &Manifest, conf: &Config, region: String, n_workers: usize) -> Result<()> {
    mass_component(mf, conf, region, UpgradeMode::UpgradeWait, n_workers)
}

/// Helm installs a service and its dependent components in parallel
///
/// Installs multiple services at a time in a threadpool.
/// This upgrade mode does not wait so this should only be limited by k8s.
pub fn install(mf: &Manifest, conf: &Config, region: String, n_workers: usize) -> Result<()> {
    mass_component(mf, conf, region, UpgradeMode::UpgradeInstall, n_workers)
}


/// Helm diff a service and its dependent components in parallel
///
/// Returns the diffs only from all services across a region.
/// Farms out the work to a thread pool.
pub fn diff(mf: &Manifest, conf: &Config, region: String, n_workers: usize) -> Result<()> {
    mass_component(mf, conf, region, UpgradeMode::DiffOnly, n_workers)
}

// Find a service and its child components and helm::parallel::upgrade them
fn mass_component(base: &Manifest, conf: &Config, region: String, umode: UpgradeMode, n_workers: usize) -> Result<()> {
    let mut svcs = vec![];
    // the versions and images must match in component deploys
    let img = base.image.clone().ok_or_else(|| ErrorKind::ManifestFailure("image".into()))?;
    let ver = base.version.clone().ok_or_else(|| ErrorKind::ManifestFailure("version".into()))?;

    if !base.external {
        svcs.push(base.clone());
    }

    for svc in &base.children {
        debug!("Scanning components: {}", svc.name);
        let mut sub = Manifest::basic(&svc.name, conf, None)?;
        let subimg = sub.image.clone().ok_or_else(|| ErrorKind::ManifestFailure("image".into()))?;
        if sub.disabled || sub.external || !sub.regions.contains(&region) {
            bail!("Components must be internal to the region and not disabled");
        }
        assert!(sub.children.is_empty()); // verify catches this
        if img != subimg {
            bail!("Component dependencies must use the same image ({} != {})", img, subimg);
        }
        // enforce using the same version
        sub.version = Some(ver.clone());
        svcs.push(sub);
    }
    parallel::upgrade_join(svcs, conf, region, umode, n_workers)
}
