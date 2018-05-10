use super::{parallel, direct, UpgradeMode};
use super::{Result, Config, Manifest, ErrorKind};


/// A smarter version of `direct::upgrade_wrapper`
///
/// If the service requested to be upgraded/diffed/installed/recreated has children:
/// we will upgrade them in parallel and join on the results.
///
/// Otherwise it defers straight tothe old `direct::upgrade_wrapper`
pub fn upgrade_wrapper(svc: &str, umode: UpgradeMode, region: &str,
                       conf: &Config, ver: Option<String>, n_workers: usize)
    -> Result<()>
{
    let mut base = Manifest::stubbed(svc, conf, region)?.set_version(&ver);
    if base.children.is_empty() {
        // NORMAL SINGLE UPGRADE
        return direct::upgrade_wrapper(svc, umode, region, conf, ver).map(|_| ())
    }
    // Set version down if override passed
    if ver.is_some() {
        base.version = ver;
    }

    // PARALLEL MODE AHEAD
    warn!("Using experimental parallel {}", umode);
    // the versions and images must match in component deploys
    let img = base.image.clone().ok_or_else(|| ErrorKind::ManifestFailure("image".into()))?;

    // start populating the vector of services
    let mut svcs = vec![];
    if !base.external {
        svcs.push(base.clone());
    }

    for svc in &base.children {
        debug!("Scanning components: {}", svc.name);
        let mut sub = Manifest::stubbed(&svc.name, conf, &region)?;
        let subimg = sub.image.clone().ok_or_else(|| ErrorKind::ManifestFailure("image".into()))?;
        if sub.disabled || sub.external || !sub.regions.contains(&region.to_string()) {
            bail!("Child component {} must exist in the same region", sub.name);
        }
        assert!(sub.children.is_empty()); // verify catches this
        if img != subimg {
            bail!("Component dependencies must use the same image ({} != {})", img, subimg);
        }
        // enforce using the same version if set
        if base.version.is_some() {
            // NB: this avoids pinned children..
            sub.version = base.version.clone();
        }
        svcs.push(sub);
    }
    parallel::upgrade_join(svcs, conf, region, umode, n_workers)
}
