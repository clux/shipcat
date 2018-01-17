/// This file contains all the hidden `shipcat list-*` subcommands

use super::Result;

/// Print the supported environments
pub fn environments() -> Result<()> {
    println!("dev");
    Ok(())
}
