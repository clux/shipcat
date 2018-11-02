use super::{Result, Manifest};
use config::{Region, ManifestDefaults};

/// Behavioural trait for where a Manifest backend gets its data from.
///
/// This abstracts the behaviour of fetching manifests either from disk or a db.
/// The following methods must be implemented:
pub trait Backend {
    /// A way to list all available services
    fn available(region: &str) -> Result<Vec<String>>;

    /// A way to generate a `Completed` Manifest
    fn completed(service: &str, defs: &ManifestDefaults, reg: &Region) -> Result<Manifest>;

    /// A way to serialize a `RawData` Manifest
    fn raw(service: &str, reg: &Region) -> Result<Manifest>;

    /// A way to generate a `Stubbed` Manifest
    /// TODO: This is only useful for the CLI.
    fn stubbed(service: &str, defs: &ManifestDefaults, reg: &Region) -> Result<Manifest>;
}
