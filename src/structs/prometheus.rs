#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Dashboard {
    /// Metric strings to track
    pub rows: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Prometheus {
    /// Whether to poll
    pub enabled: bool,
    /// Path to poll
    pub path: String,
    // TODO: Maybe include names of metrics?
}
