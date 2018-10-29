use regex::Regex;

use super::traits::Verify;
use super::{Result, Config};

// HostAlias support for all pods regardless of network configuration.

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct HostAlias {
    /// ip address string
    pub ip: String,
    /// add additional entries that resolve the ip address to the hosts file
    pub hostnames: Vec<String>,
}

impl Verify for HostAlias {
    //  only verify syntax
    fn verify(&self, _: &Config) -> Result<()> {
        // Commonly accepted hostname regex from https://stackoverflow.com/questions/106179/regular-expression-to-match-dns-hostname-or-ip-address
        let ip_re = Regex::new(r"^(([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])\.){3}([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])$").unwrap();
        if self.ip == "" || !ip_re.is_match(&self.ip){
            bail!("The ip address for the host alias is incorrect");
        }
        if self.hostnames.is_empty() {
            bail!("At least one hostname must be specified for the host alias");
        }
        for hostname in &self.hostnames {
            // Commonly accepted ip address regex from https://stackoverflow.com/questions/106179/regular-expression-to-match-dns-hostname-or-ip-address
            let host_re = Regex::new(r"^(([a-zA-Z0-9]|[a-zA-Z0-9][a-zA-Z0-9\-]*[a-zA-Z0-9])\.)*([A-Za-z0-9]|[A-Za-z0-9][A-Za-z0-9\-]*[A-Za-z0-9])$").unwrap();
            if !host_re.is_match(&hostname) {
                bail!("The hostname {} is incorrect for {}", hostname, self.ip);
            }
        }
        Ok(())
    }
}
