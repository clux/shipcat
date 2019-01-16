use super::{Result, Manifest, Region};
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
    #[cfg(not(feature = "filesystem"))]
    fn default() -> Self { ManifestType::Base }
    #[cfg(feature = "filesystem")]
    fn default() -> Self { ManifestType::SingleFile }
}

/// This library defines the way to upgrade a manifest from Base
/// but each backend has to implement its own way of:
/// - listing services from its backing
/// - creating a base manifest from its backing
impl Manifest {
    /// Upgrade a `Base` manifest to either a Complete or a Stubbed one
    fn upgrade(mut self, reg: &Region, kind: ManifestType) -> Result<Self> {
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
        Ok(self)
    }

    /// Complete a Base manifest with stub secrets
    pub fn stub(self, reg: &Region) -> Result<Self> {
        self.upgrade(reg, ManifestType::Stubbed)
    }

    /// Complete a Base manifest with actual secrets
    pub fn complete(self, reg: &Region) -> Result<Self> {
        self.upgrade(reg, ManifestType::Completed)
    }
}


/// Various states a Config can exist in depending on resolution.
///
/// This only matters within shipcat and is used to optimize speed of accessors.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ConfigType {
    /// The full config, region-independent, resolved secrets
    // Completed,

    /// A filtered config for a specific region, with resolved secrets
    Filtered,

    /// A config with a single region entry with blank secrets
    ///
    /// Same as what's on disk, secrets unresolved, but only one region.
    /// This is the CRD equivalent.
    Base,

    /// The full config as read from disk. Secrets unresolved
    #[cfg(feature = "filesystem")]
    File,
}

/// Default is the feature-specified base type to force constructors into chosing.
///
/// This relies on serde default to populate on deserialize from disk/crd.
impl Default for ConfigType {
    #[cfg(feature = "filesystem")]
    fn default() -> Self { ConfigType::File }
    #[cfg(not(feature = "filesystem"))]
    fn default() -> Self { ConfigType::Base }
}
