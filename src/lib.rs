#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;

#[macro_use]
extern crate log;

#[macro_use]
extern crate failure;

use failure::Error;
pub type BabylResult<T> = Result<T, Error>;

mod manifest;
pub use manifest::{init, validate, Manifest};
