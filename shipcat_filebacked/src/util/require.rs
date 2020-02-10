use shipcat_definitions::Result;

pub trait Require<T> {
    fn require(self, name: &str) -> Result<T>;
}

impl<T> Require<T> for Option<T> {
    fn require(self, name: &str) -> Result<T> {
        match self {
            Some(t) => Ok(t),
            None => bail!("{} is required", name),
        }
    }
}
