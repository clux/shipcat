#![allow(non_snake_case)]

/// Allow normal error handling from structs
pub use super::{Result, ErrorKind, Error};

pub use super::config::{Config, VaultConfig};

/// Products needs some structs
pub use super::structs;


/// main module
pub mod product;

// Exports
pub use self::product::Product;

pub use self::product::{show, validate};

// private module to define merge behaviour
mod merge;
