#![recursion_limit = "1024"]


#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;

#[macro_use]
extern crate tera;


#[macro_use]
extern crate log;


#[macro_use]
extern crate error_chain;
error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }
    links {}
    foreign_links {
        Fmt(::std::fmt::Error);
        Io(::std::io::Error) #[cfg(unix)];
        Float(::std::num::ParseFloatError);
        Int(::std::num::ParseIntError);
        Serde(serde_yaml::Error);
    }
    errors {
        UnknownToolchainVersion(v: String) {
            description("unknown toolchain version"),
            display("unknown toolchain version: '{}'", v),
        }
    }
}


mod manifest;
pub use manifest::{init, validate, Manifest};

mod kube;
pub use kube::generate;
