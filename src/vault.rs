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
            let path = env::home_dir()
                .ok_or_else(|| { ErrorKind::NoHomeDirectory })?
                .join(".vault-token");

            // Read the file.
            let mut f = File::open(path)?;
            let mut token = String::new();
            f.read_to_string(&mut token)?;
            Ok(token)
        })
        .chain_err(|| ErrorKind::MissingVaultToken)
}

/// Secret data retrieved from Vault using only standard fields
#[derive(Debug, Deserialize)]
struct Secret {
    /// The key-value pairs associated with this secret.
    data: BTreeMap<String, String>,
    // How long this secret will remain valid for, in seconds.
    lease_duration: u64,
}

/// List data retrieved from Vault when listing available secrets
#[derive(Debug, Deserialize)]
struct ListSecrets {
    data: BTreeMap<String, Vec<String>>
}

/// Vault client with cached data
pub struct Vault {
    /// Our HTTP client.  This can be configured to mock out the network.
    client: reqwest::Client,
    /// The address of our Vault server.
    addr: reqwest::Url,
    /// The token which we'll use to access Vault.
    token: String,
    /// Vault operation mode
    mode: Mode,
}

/// Vault usage mode
#[derive(PartialEq)]
pub enum Mode {
    /// Normal HTTP calls to vault returing actual secret
    Standard,
    /// Avoiding HTTP calls to vault altogether
    Mocked,
    /// Using HTTP calls but masking secrets
    Masked,
}

impl Vault {
    /// Initialize using the same evars or token files that the `vault` CLI uses
    pub fn default() -> Result<Vault> {
        Vault::new(reqwest::Client::new(), &default_addr()?, default_token()?, Mode::Standard)
    }
    /// Initialize using vault evars, but return masked return values
    pub fn masked() -> Result<Vault> {
        Vault::new(reqwest::Client::new(), &default_addr()?, default_token()?, Mode::Masked)
    }
    /// Initialize using a dud client (still needs an addr)
    pub fn mocked() -> Result<Vault> {
        Vault::new(reqwest::Client::new(), &default_addr()?, "", Mode::Mocked)
    }

    fn new<U, S>(client: reqwest::Client, addr: U, token: S, mode: Mode) -> Result<Vault>
        where U: reqwest::IntoUrl,
              S: Into<String>
    {
        let addr = addr.into_url()?;
        Ok(Vault { client, addr, mode, token: token.into() })
    }

    // The actual HTTP GET logic
    fn get_secret(&self, path: &str) -> Result<Secret> {
        let url = self.addr.join(&format!("v1/{}", path))?;
        debug!("GET {}", url);

        let mkerr = || ErrorKind::Url(url.clone());
        let mut res = self.client.get(url.clone())
            // Leaving the connection open will cause errors on reconnect
            // after inactivity.
            .header(Connection::close())
            .header(XVaultToken(self.token.clone()))
            .send()
            .chain_err(&mkerr)?;

        // Generate informative errors for HTTP failures, because these can
        // be caused by everything from bad URLs to overly restrictive vault policies
        if !res.status().is_success() {
            let status = res.status().to_owned();
            let err: Error = ErrorKind::UnexpectedHttpStatus(status).into();
            return Err(err).chain_err(&mkerr);
        }

        let mut body = String::new();
        res.read_to_string(&mut body)?;
        Ok(serde_json::from_str(&body)?)
    }

    /// List secrets
    ///
    /// Does a HTTP LIST on the folder a service is in and returns the keys
    pub fn list(&self, path: &str) -> Result<Vec<String>> {
        let url = self.addr.join(&format!("v1/secret/{}?list=true", path))?;
        debug!("LIST {}", url);

        let mkerr = || ErrorKind::Url(url.clone());
        let mut res = self.client.get(url.clone())
            // Leaving the connection open will cause errors on reconnect
            // after inactivity.
            .header(Connection::close())
            .header(XVaultToken(self.token.clone()))
            .send()
            .chain_err(&mkerr)?;

        // Generate informative errors for HTTP failures, because these can
        // be caused by everything from bad URLs to overly restrictive vault policies
        if !res.status().is_success() {
            let status = res.status().to_owned();
            let err: Error = ErrorKind::UnexpectedHttpStatus(status).into();
            return Err(err).chain_err(&mkerr);
        }

        let mut body = String::new();
        res.read_to_string(&mut body)?;
        let lsec : ListSecrets = serde_json::from_str(&body)?;
        if !lsec.data.contains_key("keys") {
            return Err(ErrorKind::MissingSecret(url.to_string()).into());
        }
        let res = lsec.data["keys"].iter()
            .filter(|e| !e.ends_with("/")) // skip sub folders
            .map(|e| e.to_string())
            .collect::<Vec<String>>();
        Ok(res)
    }


    /// Read secret from a Vault via an authenticated HTTP GET (or memory cache)
    pub fn read(&self, key: &str) -> Result<String> {
        let pth = format!("secret/{}", key);
        if self.mode == Mode::Mocked {
            return Ok("VAULT_MOCKED".into());
        }
        let secret = match self.get_secret(&pth) {
            Ok(s) => s,
            Err(e) => {
                error!("{}", e);
                return Err(ErrorKind::MissingSecret(pth).into())
            }
        };

        // NB: Currently assume each path in vault has a single `value`
        // Read the value key (which should exist)
        secret.data
            .get("value")
            .ok_or_else(|| { ErrorKind::MissingSecret(pth).into() })
            .map(|v| {
                if self.mode == Mode::Masked {
                    "VAULT_VALIDATED".into()
                } else {
                    v.clone()
                }
            })
    }
}


#[cfg(test)]
mod tests {
    use super::Vault;

    #[test]
    fn get_dev_secret() {
        let client = Vault::default().unwrap();
        let secret = client.read("dev-uk/test-shipcat/FAKE_SECRET").unwrap();
        assert_eq!(secret, "hello");
    }

    #[test]
    fn list_dev_secrets() {
        let client = Vault::default().unwrap();
        let secrets = client.list("dev-uk/test-shipcat").unwrap();
        assert_eq!(secrets, vec!["FAKE_SECRET".to_string()]);
    }
}
