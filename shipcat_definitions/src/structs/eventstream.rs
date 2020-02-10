use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EventDefinition {
    pub key: String,
    pub value: String,
}


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventStream {
    pub name: String,

    pub producers: Vec<String>,
    pub consumers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_definitions: Option<Vec<EventDefinition>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<BTreeMap<String, String>>,
}



