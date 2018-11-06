use super::{Result, Manifest};
use config::{Region, Config};
use super::vault::Vault;


/// Various states a manifest can exist in depending on resolution.
///
/// This only matters within shipcat and is used to optimize speed of accessors.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ManifestType {
    /// A completed manifest
    ///
    /// The version that is fully ready to pass to `helm template`, i.e.:
    /// - region overrides accounted for
    /// - evars are templated
    /// - secrets are available
    /// - configs inlined and templated with secrets
    ///
    /// This is the `shipcat values -s` equivalent of a manifest.
    ///
    /// A `Base` Manifest can become a `Completed` Manifest by:
    /// - templating evars
    /// - evaluating secrets
    /// - templating configs
    Completed,

    /// A stubbed manifest
    ///
    /// Indistinguishable from a `Completed` manifest except from secrets:
    /// - secrets populated with garbage values (not resolved from vault)
    /// - configs templated with garbage secret values
    /// - evars templated with garbage secret values
    ///
    /// This is the `shipcat values` equivalent of a manifest.
    Stubbed,

    /// The Base manifest
    ///
    /// A state that is upgradeable to a completed one, contains all the pieces,
    /// but does not have any secrets. This form is used internally in the cli,
    /// as well as in the cluster as the canonical CRD state.
    ///
    /// Major features:
    /// - region overrides accounted for
    /// - templates (configs + evars) left in template form
    /// - secrets unresolved
    ///
    /// This is the CRD equivalent of a manifest.
    /// It's important that the CRD equivalent abstracts away config files, but not secrets.
    /// Thus files have to be read, and not templated for this, then shipped off to kube.
    Base,

    /// A Simplified manifest
    ///
    /// Equivalent to a Base manifest but no configs read.
    /// This is faster to retrieve from disk.
    /// This type CANNOT be upgraded to Stubbed/Completed.
    #[cfg(feature = "filesystem")]
    Simple,

    /// A Manifest File
    ///
    /// This is an unmerged file, and should not be used for anything except merging.
    #[cfg(feature = "filesystem")]
    SingleFile,
}

/// Default is the feature-specified base type to force constructors into chosing.
///
/// This relies on serde default to populate on deserialize from disk/crd.
impl Default for ManifestType {
    fn default() -> Self {
        if cfg!(feature = "filesystem") {
            #[cfg(feature = "filesystem")]
            return ManifestType::SingleFile;
        }
        ManifestType::Base
    }
}

/// Behavioural trait for where a Manifest backend gets its data from.
///
/// This abstracts the behaviour of fetching manifests either from disk or a db.
/// See `ManifestType` for information about what the types are expected to contain.
/// This trait is only used internally.
pub trait Backend {
    /// A way to create a `Base` manifest
    fn _base(service: &str, conf: &Config, reg: &Region) -> Result<Manifest>;

    /// A way to list all available services in the region
    fn _available(region: &str) -> Result<Vec<String>>;
}

#[cfg(any(feature = "filesystem", feature = "crd"))]
impl Manifest where Manifest: Backend {
    /// List all services available in a region
    pub fn available(region: &str) -> Result<Vec<String>> {
        Manifest::_available(region)
    }

    /// Create a `Base` manifest
    pub fn base(service: &str, conf: &Config, reg: &Region) -> Result<Manifest> {
        Manifest::_base(service, conf, reg)
    }
    /// Upgrade a `Base` manifest to either a Complete or a Stubbed one
    pub fn upgrade(&mut self, reg: &Region, kind: ManifestType) -> Result<()> {
        assert_eq!(self.kind, ManifestType::Base); // sanity
        let v = match kind {
            ManifestType::Completed => Vault::regional(&reg.vault)?,
            ManifestType::Stubbed => Vault::mocked(&reg.vault)?,
            _ => bail!("Can only upgrade a Base manifest to Completed or Stubbed"),
        };
        // replace one-off templates in evar strings with values
        // note that this happens before secrets because:
        // secrets may be injected at this step from the Region
        self.template_evars(reg)?;
        // secrets before configs (.j2 template files use raw secret values)
        self.secrets(&v, &reg.vault)?;

        // templates last
        self.template_configs(reg)?;
        self.kind = kind;
        Ok(())
    }

    /// Create a completed manifest with stubbed secrets (faster to retrieve)
    pub fn stubbed(service: &str, conf: &Config, reg: &Region) -> Result<Manifest> {
        let mut mf = Manifest::base(service, &conf, reg)?;
        mf.upgrade(reg, ManifestType::Stubbed)?;
        Ok(mf)
    }

    /// Create a completed manifest fetching secrets from Vault
    pub fn completed(service: &str, conf: &Config, reg: &Region) -> Result<Manifest> {
        let mut mf = Manifest::base(service, &conf, reg)?;
        mf.upgrade(reg, ManifestType::Completed)?;
        Ok(mf)
    }
}
