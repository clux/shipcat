/// This file contains all the hidden `shipcat list-*` subcommands

use super::Result;

/// Print the supported environments
pub fn environments() -> Result<()> {
    // TODO: look for override files in the environments folder!
    println!("dev-uk");
    println!("dev-global1");
    println!("dev-ops");
    Ok(())
}
