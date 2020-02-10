//! Interface to self_update crate for auto-upgrading shipcat

use super::Result;
use semver::Version;
use std::{io::Read, path::PathBuf};


fn get_target() -> Result<String> {
    if !cfg!(target_arch = "x86_64") {
        bail!("shipcat is only built for 64 bit architectures");
    }
    let arch = "x86_64";

    let os_config = (cfg!(target_os = "linux"), cfg!(target_os = "macos"));
    let os = match os_config {
        (true, _) => "unknown-linux-musl",
        (_, true) => "apple-darwin",
        _ => bail!("shipcat only has assets for mac and linux"),
    };

    Ok(format!("{}-{}", arch, os))
}

#[derive(Deserialize)]
struct GithubReleaseInfo {
    browser_download_url: String,
}

fn find_actual_dl_url(url: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let mut req = client.get(url);
    if let Ok(token) = std::env::var("SHIPCAT_AUTOUPGRADE_TOKEN") {
        req = req.bearer_auth(token);
    }
    let mut res = req.send()?;

    // Generate informative errors for HTTP failures
    if !res.status().is_success() {
        let status = res.status().to_owned();
        bail!("Failed to get {}: {}", url, status)
    }

    let mut body = String::new();
    res.read_to_string(&mut body)?;
    let ghi: GithubReleaseInfo = serde_json::from_str(&body)?;
    Ok(ghi.browser_download_url)
}

struct ExeInfo {
    /// Path to current_exe
    path: PathBuf,
    /// Best guess at install prefix based on path (only for static executables)
    base: PathBuf,
}

fn identify_exe() -> Result<ExeInfo> {
    let pth = std::env::current_exe()?;
    trace!("shipcat at {}", pth.display());
    Ok(ExeInfo {
        path: pth.clone(),
        base: pth.parent().expect("current_exe has a parent dir").to_path_buf(),
    })
}

/// Attempt to upgrade shipcat
pub fn self_upgrade(ver: Option<Version>) -> Result<()> {
    debug!("self_upgrade to pin={:?}", ver);

    // self_update crate returns &mut self in builder... wtf
    let releases = if let Ok(token) = std::env::var("SHIPCAT_AUTOUPGRADE_TOKEN") {
        self_update::backends::github::ReleaseList::configure()
            .repo_owner("babylonhealth")
            .repo_name("shipcat")
            .auth_token(&token)
            .build()?
            .fetch()?
    } else {
        self_update::backends::github::ReleaseList::configure()
            .repo_owner("babylonhealth")
            .repo_name("shipcat")
            .build()?
            .fetch()?
    };
    trace!("found releases:");
    trace!("{:#?}\n", releases);

    // If using a requested version, we have to find that
    let rel = if let Some(v) = &ver {
        releases
            .into_iter()
            .find(|r| Version::parse(&r.tag) == Ok(v.clone()))
    } else {
        // pick latest if upgrading opportunistically
        releases.into_iter().find(|r| !r.assets.is_empty())
    };

    let release = match rel {
        Some(r) => r,
        None => bail!("No matching shipcat release found for version: {:?}", ver),
    };
    let asset = release
        .asset_for(&get_target()?)
        .expect("Asset for valid target exists");
    debug!("selected {:?}", asset);
    if Version::parse(&release.tag) == Version::parse(env!("CARGO_PKG_VERSION")) {
        // NB: Config::verify_version checks for >=, allow non-noop upgrades here
        info!("shipcat is already running {}", release.tag);
        return Ok(());
    }
    info!("upgrading from {} to {}", env!("CARGO_PKG_VERSION"), release.tag);

    // using temp dir ends up causing tons of cross-link failures (18) or os error 26
    // basically; fs::rename across partition causes many problems...
    // we instead make a directory where shipcat lives, then clean that up...
    let exe = identify_exe()?;

    // Because upgrade fs::rename call can fail when moving across partitions
    // we try to avoid this by using a temp dir inside the path shipcat is found..
    let tmp_dir = self_update::TempDir::new_in(exe.base, "shipcat_upgrade")?;
    std::fs::create_dir_all(tmp_dir.path())?;
    debug!("using tmp_dir: {:?}", tmp_dir);

    let tmp_tarball_path = tmp_dir.path().join(&asset.name);
    debug!("tarball path: {:?}", tmp_tarball_path);

    let tmp_tarball = std::fs::File::create(&tmp_tarball_path)?;
    debug!("tmp tarball: {:?}", tmp_tarball);

    // For some reason we have an extra layer of indirection here..
    // asset_url is just a link to where we can get the actual tarball..
    let dl_url = find_actual_dl_url(&asset.download_url)?;
    debug!("upgrading from {}", dl_url);

    self_update::Download::from_url(&dl_url)
        .show_progress(true)
        .download_to(&tmp_tarball)?;

    let bin_name = std::path::PathBuf::from("bin/shipcat");
    self_update::Extract::from_source(&tmp_tarball_path)
        .archive(self_update::ArchiveKind::Tar(Some(self_update::Compression::Gz)))
        .extract_file(&tmp_dir.path(), &bin_name)?;

    debug!("Replacing {:?}", exe.path);
    let swap = tmp_dir.path().join("replacement");
    let src = tmp_dir.path().join("bin").join("shipcat");
    let dest = exe.path;

    debug!("Backing up {} to {}", dest.display(), swap.display());
    std::fs::rename(&dest, &swap)?;

    debug!("Swapping in new version of shipcat {:?} -> {:?}", src, dest);
    if let Err(e) = std::fs::rename(&src, &dest) {
        warn!("rollback rename: {:?} -> {:?} due to {}", swap, dest, e);
        std::fs::rename(&swap, &dest)?; // fallback on error TODO: ?
    }
    Ok(())
}
