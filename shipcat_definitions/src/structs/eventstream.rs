use super::Result;
use std::collections::BTreeMap;

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "filesystem", serde(deny_unknown_fields))]
pub struct EventDefinition {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventStream {
    pub name: String,

    #[serde(default)]
    pub producers: Vec<String>,

    #[serde(default)]
    pub consumers: Vec<String>,

    pub event_definitions: Vec<EventDefinition>,

    #[serde(default)]
    pub config: BTreeMap<String, String>,
}

impl EventStream {
    pub fn verify(&self) -> Result<()> {
        if self.event_definitions.is_empty() {
            bail!("Event definitions must not be empty when EventStreams is specified");
        }
        if self.name.is_empty() {
            bail!("EventStream name must not be empty when EventStreams is specified");
        }
        Ok(())
    }
}
