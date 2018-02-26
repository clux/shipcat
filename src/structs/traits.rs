use super::Result;

pub trait Verify {
    /// Verifying that this struct is sane
    ///
    /// NB: This is called after defaults and implicits are filled in.
    fn verify(&self) -> Result<()>;
}
