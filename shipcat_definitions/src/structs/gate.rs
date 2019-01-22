use std::ops::Not;

/// Gate setup for a service
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Gate {
    /// let external traffic in or not
    #[serde(default, skip_serializing_if = "Not::not")]
    pub public: bool,

    /// allow connection upgrade to websockets
    #[serde(default, skip_serializing_if = "Not::not")]
    pub websockets: bool,
}
