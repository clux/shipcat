use super::structs::{Kong};

pub struct SimpleManifest {
    pub name: String,
    pub version: Option<String>,
    pub image: Option<String>,
    pub kong: Option<Kong>,
}
