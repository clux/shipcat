/// Configuration for authorization of requests
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Authorization {
    /// Allowed values for the `aud` claim of the JWT payload.
    pub allowed_audiences: Vec<String>,

    /// Are anonymous requests allowed to reach the service?
    ///
    /// If true, requests with no `Authorization` header (or an invalid/expired JWT, if allow_invalid_tokens is true) will be proxied to the service (but will receive an `X-Anonymous-Consumer: true` header)
    /// If false, they will be rejected (with a 401 response)
    pub allow_anonymous: bool,

    /// Are requests with invalid/expired tokens allowed to reach the service?
    ///
    /// If true, Kong will allow requests with invalid `Authorization` headers.
    pub allow_invalid_tokens: bool,

    /// What JWT scopes are required for the service?
    ///
    /// If the JWT does not contain the required scopes, the request will be rejected with a 401.
    pub required_scopes: Vec<String>,

    /// Are tokens in cookies allowed
    ///
    /// If true, CSRF protection is enabled and access tokens are extracted from cookies.
    pub allow_cookies: bool,
}
