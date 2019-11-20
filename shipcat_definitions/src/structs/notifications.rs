/// Modes for slack upgrade notifications in this region
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum NotificationMode {
    /// Do not notify on upgrades in this region
    Silent,

    /// Print a basic message in the configured notification channel
    MessageOnly,

    /// Print a basic message and also tag all maintainers in the message (default)
    NotifyMaintainers,
}
impl Default for NotificationMode {
    fn default() -> Self {
        Self::NotifyMaintainers
    }
}
