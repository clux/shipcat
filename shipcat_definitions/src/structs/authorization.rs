/// Configuration for authorization of requests
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Authorization {
    /// Allowed values for the `aud` claim of the JWT payload.
    pub allowed_audiences: Vec<String>,

    /// Are anonymous requests allowed to reach the service?
    ///
    /// If true, requests with no `Authorization` header (or an invalid/expired JWT) will be proxied to the service (but will receive an `Anonymous: true` header)
    /// If false, they will be rejected (with a 401 response)
    pub allow_anonymous: bool,

    /// Should invalid/expired tokens be stripped from the upstream request?
    ///
    /// If true, invalid `Authorization` headers will be removed from the request.
    pub remove_invalid_tokens: bool,

    /// What JWT scopes are required for the service?
    ///
    /// If the JWT does not contain the required scopes, the request will be rejected with a 401.
    pub required_scopes: Vec<String>,

    /// Are tokens in cookies allowed
    ///
    /// If true, CSRF protection is enabled and access tokens are extracted from cookies.
    pub allow_cookies: bool,
}
