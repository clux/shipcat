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

    /// Should expired access_tokens in the Cookie header be refreshed automatically through an internal auth service?
    ///
    /// If true, the cookie is parsed, its expiry is checked, and (if expired) it is replaced with a fresh access_token.
    /// A new cookie pair is sent through a Set-Cookie header.
    pub enable_cookie_refresh: bool,

    /// URL of authentication service where cookie_refresh is performed
    /// e.g. "http://ai-auth/v1/authenticate"
    pub refresh_auth_service: Option<String>,

    /// The refresh token is posted to the refresh_auth_service as a JSON object with a single key (this field).
    /// e.g. "api_key" will result in the following body: {"api_key": "asdf1234"}
    pub refresh_body_refresh_token_key: Option<String>,

    //  Defines the max_age parameter of the new HTTP cookie
    pub refresh_max_age_sec: Option<u32>,

    /// HTTP timeout for cookie refresh in msec
    pub refresh_http_timeout_msec: Option<u32>,

    /// How many seconds before their expiry should we refresh the tokens
    pub refresh_renew_before_expiry_sec: Option<u32>,
}
