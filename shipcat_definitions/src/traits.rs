use super::{Result, Manifest, ManifestType, Config};
use config::{Region, ManifestDefaults};

/// Behavioural trait for where a Manifest backend gets its data from.
///
/// This abstracts the behaviour of fetching manifests either from disk or a db.
/// The following methods must be implemented:
pub trait Backend {
    /// A way to serialize a `Base` Manifest
    ///
    /// Note that this type has strictly more information than a filesystem Manifest.
    /// A filesystem backed one will have to merge and read config files first.
    /// A CRD based one is expected to exist in this state.
    fn base(service: &str, reg: &Region) -> Result<Manifest>;

    /// A way to list all available services
    fn available(region: &str) -> Result<Vec<String>>;

    /// A way to generate a `Completed` Manifest
    fn completed(service: &str, defs: &ManifestDefaults, reg: &Region) -> Result<Manifest>;

    /// A way to generate a `Stubbed` Manifest
    fn stubbed(service: &str, defs: &ManifestDefaults, reg: &Region) -> Result<Manifest>;

    /// A way to upgrade a `Base` manifest
    ///
    /// The kind to upgrade to must be either `Completed` or `Stubbed`.
    fn upgrade(&mut self, defs: &ManifestDefaults, reg: &Region, kind: ManifestType) -> Result<()>;

    /// Verify that a Manifest is valid in the current backend
    ///
    /// What valid means depends on the implementation in the Backend
    fn validate(svc: &str, conf: &Config, reg: &Region, secrets: bool) -> Result<()>;
}
