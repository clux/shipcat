//! A very basic client for Hashicorp's Vault

use reqwest;
use reqwest::header::Connection;
use serde_json;
use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::Read;

use super::{Result, ErrorKind, ResultExt, Error};

// hyper/reqwest header for Vault GET requests
header! { (XVaultToken, "X-Vault-Token") => [String] }

fn default_addr() -> Result<String> {
    env::var("VAULT_ADDR").map_err(|_| ErrorKind::MissingVaultAddr.into())
}
fn default_token() -> Result<String> {
    env::var("VAULT_TOKEN")
        .or_else(|_: env::VarError| -> Result<String> {
            // Build a path to ~/.vault-token.
            let mut path = env::home_dir().ok_or_else(|| { ErrorKind::NoHomeDirectory })?;
            path.push(".vault-token");

            // Read the file.
            let mut f = File::open(path)?;
            let mut token = String::new();
            f.read_to_string(&mut token)?;
            Ok(token)
        })
        .chain_err(|| ErrorKind::MissingVaultToken)
}

/// Secret data retrieved from Vault.  This has a bunch more fields, but
/// the exact list of fields doesn't seem to be documented anywhere, so
/// let's be conservative.
#[derive(Debug, Deserialize)]
struct Secret {
    /// The key-value pairs associated with this secret.
    data: BTreeMap<String, String>,
    // How long this secret will remain valid for, in seconds.
    lease_duration: u64,
}

/// A basic Vault client.
pub struct Client {
    /// Our HTTP client.  This can be configured to mock out the network.
    client: reqwest::Client,
    /// The address of our Vault server.
    addr: reqwest::Url,
    /// The token which we'll use to access Vault.
    token: String,
    /// Local cache of secrets.
    secrets: BTreeMap<String, Secret>,
}


impl Client {
    /// Has the user indicated that they want to enable our Vault backend?
    pub fn is_enabled() -> bool {
        default_addr().is_ok()
    }

    /// Construct a new vault::Client, attempting to use the same
    /// environment variables and files used by the `vault` CLI tool and
    /// the Ruby `vault` gem.
    pub fn default() -> Result<Client> {
        let client = reqwest::Client::new();
        Client::new(client, &default_addr()?, default_token()?)
    }

    fn new<U, S>(client: reqwest::Client, addr: U, token: S) -> Result<Client>
        where U: reqwest::IntoUrl,
              S: Into<String>
    {
        let addr = addr.into_url()?;
        Ok(Client {
            client: client,
            addr: addr,
            token: token.into(),
            secrets: BTreeMap::new(),
        })
    }

    fn get_secret(&self, path: &str) -> Result<Secret> {
        let url = self.addr.join(&format!("v1/{}", path))?;
        debug!("Getting secret {}", url);

        let mkerr = || ErrorKind::Url(url.clone());
        let mut res = self.client.get(url.clone())
            // Leaving the connection open will cause errors on reconnect
            // after inactivity.
            .header(Connection::close())
            .header(XVaultToken(self.token.clone()))
            .send()
            .chain_err(&mkerr)?;

        // Generate informative errors for HTTP failures, because these can
        // be caused by everything from bad URLs to overly restrictive
        // vault policies.
        if !res.status().is_success() {
            let status = res.status().to_owned();
            let err: Error = ErrorKind::UnexpectedHttpStatus(status).into();
            return Err(err).chain_err(&mkerr);
        }

        let mut body = String::new();
        res.read_to_string(&mut body)?;
        Ok(serde_json::from_str(&body)?)
    }

    pub fn get_loc(&mut self, pth: &str) -> Result<String> {
        // Look up cached secret or query vault:
        if !self.secrets.contains_key(pth) {
            let secret = self.get_secret(pth)?;
            self.secrets.insert(pth.to_owned(), secret);
        }

        // Get the secret from our cache - get_secret succeeded so can unwrap.
        let secret = self.secrets.get(pth).unwrap();

        // Look up the specified key in our secret's data bag.
        secret.data
            .get("value")
            .ok_or_else(|| { ErrorKind::MissingKeyInSecret(pth.to_owned()).into() })
            .map(|v| v.clone())
    }
}


#[cfg(test)]
mod tests {
    use super::Client;

    #[test]
    fn get_dev_secret() {
        let mut client = Client::default().unwrap();
        let secret = client.get_loc("secret/development/babylon_core_ruby/internal_service_auth_key").unwrap();
        assert_eq!(secret, "INTERNAL_SERVICE_DUMMY_AUTH_KEY");
    }

}
