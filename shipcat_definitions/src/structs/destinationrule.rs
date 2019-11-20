use regex::Regex;
use super::Result;

/// DestinationRule
///
/// An abstraction that captures the information needed to make routing decisions.
#[derive(Serialize, Deserialize, Clone)]
pub struct DestinationRule {

    /// The identifier the incoming request must possess to be considered for forwarding
    pub identifier: String,

    /// The host to forward this request to
    pub host: String,
}

impl DestinationRule {

    const DNS_LABEL_MAX_LENGTH: u8 = 63;
    const DNS_LABEL_PATTERN: &'static str = "^(?:[A-Za-z0-9][-A-Za-z0-9_\\.]*)?[A-Za-z0-9]$";

    fn is_dns_label(value: &str) -> bool {
        value.len() <= (DestinationRule::DNS_LABEL_MAX_LENGTH as usize) &&
            Regex::new(DestinationRule::DNS_LABEL_PATTERN).unwrap().is_match(value)
    }

    pub fn verify(&self, identifierPattern: &Regex) -> Result<bool> {
        let mut erroneous = false;

        if !identifierPattern.is_match(&self.identifier) {
            erroneous = true;
            error!("Identifier \"{}\" is invalid.", &self.identifier);
        }

        if !DestinationRule::is_dns_label(&self.host[..]) {
            erroneous = true;
            error!("Host \"{}\" is not a valid DNS label.", &self.host);
        }

        if erroneous {
            bail!("Fatal errors detected.")
        } else {
            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DestinationRule};
    use regex::Regex;

    const MATCH_ALL_PATTERN: &'static str = ".*";
    const MATCH_NONE_PATTERN: &'static str = ".^";
    const HOST_VALID: &'static str = "hostname.local";
    const HOST_INVALID: &'static str = ">:(";
    const IDENTIFIER: &'static str = "IDENTIFIER";

    fn withIdentifierAndHost(host: &str) -> DestinationRule {
        let identifier = IDENTIFIER.into();
        let host = host.into();
        DestinationRule {
            identifier,
            host,
        }
    }

    #[test]
    fn verifies_if_all_fields_valid() {
        assert!(
            withIdentifierAndHost(HOST_VALID)
                .verify(&Regex::new(MATCH_ALL_PATTERN).unwrap())
                .unwrap());
    }

    #[test]
    fn verification_fails_if_identifier_invalid() {
        withIdentifierAndHost(HOST_VALID)
            .verify(&Regex::new(MATCH_NONE_PATTERN).unwrap())
            .unwrap_err();
    }

    #[test]
    fn verification_fails_if_host_invalid() {
        withIdentifierAndHost(HOST_INVALID)
            .verify(&Regex::new(MATCH_ALL_PATTERN).unwrap())
            .unwrap_err();
    }

    #[test]
    fn verification_fails_if_all_fields_invalid() {
        withIdentifierAndHost(HOST_INVALID)
            .verify(&Regex::new(MATCH_NONE_PATTERN).unwrap())
            .unwrap_err();
    }
}
