use super::{Result, Manifest, ManifestType};
use config::{Region, Config};

/// Behavioural trait for where a Manifest backend gets its data from.
///
/// This abstracts the behaviour of fetching manifests either from disk or a db.
/// The following methods must be implemented:
pub trait Backend {
    /// A way to create a `Base` manifest
    ///
    /// Note that this type has strictly more information than a filesystem Manifest.
    /// A filesystem backed one will have to merge and read config files first.
    /// A CRD based one is expected to exist in this state.
    fn base(service: &str, conf: &Config, reg: &Region) -> Result<Manifest>;

    /// A way to create a `Simple` manifest
    ///
    /// This is a type that is only used for reducing internally.
    /// It contains strictly less than a `Base` manifest, but cannot be upgraded.
    fn simple(service: &str, conf: &Config, reg: &Region) -> Result<Manifest> ;

    /// A way to create a `SingleFile` manifest
    ///
    /// This really only reads from disk. Can ONLY be relied upon to read globals.
    /// This means: name, regions, metadata
    fn blank(service: &str) -> Result<Manifest>;

    /// A way to list all available services in the region
    fn available(region: &str) -> Result<Vec<String>>;

    /// A way to list all available services
    ///
    /// This may be more than available, or may not, depending on the backing.
    fn all() -> Result<Vec<String>>;

    /// A way to generate a `Completed` Manifest
    fn completed(service: &str, conf: &Config, reg: &Region) -> Result<Manifest>;

    /// A way to generate a `Stubbed` Manifest
    fn stubbed(service: &str, conf: &Config, reg: &Region) -> Result<Manifest>;

    /// A way to upgrade a `Base` manifest
    ///
    /// The kind to upgrade to must be either `Completed` or `Stubbed`.
    fn upgrade(&mut self, reg: &Region, kind: ManifestType) -> Result<()>;

    /// Verify that a Manifest is valid in the current backend
    ///
    /// What valid means depends on the implementation in the Backend
    fn validate(svc: &str, conf: &Config, reg: &Region, secrets: bool) -> Result<()>;
}
