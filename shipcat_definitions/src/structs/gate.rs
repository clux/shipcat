use std::ops::Not;

/// Gate service configuration
///
/// Gate is a babylon-specific, filtering entry-point for kong, as such, requires kong.
/// Configuration for gate is expected to be picked up outside of shipcat for services using kong.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Gate {
    /// Let external traffic in or not
    #[serde(default, skip_serializing_if = "Not::not")]
    pub public: bool,

    /// Allow connection upgrade to websockets
    #[serde(default, skip_serializing_if = "Not::not")]
    pub websockets: bool,
}
