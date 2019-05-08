use shipcat_definitions::{Result};
use shipcat_definitions::structs::resources::{Resources, ResourceRequest};
use shipcat_definitions::deserializers::{RelaxedString};

use crate::util::{Build, Require};

#[derive(Deserialize, Clone, Default)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceRequirementsSource {
    pub requests: ResourcesSource,
    pub limits: ResourcesSource,
}

impl Build<Resources<String>, ()> for ResourceRequirementsSource {
    fn build(self, params: &()) -> Result<Resources<String>> {
        let resources = Resources {
            requests: self.requests.build(params)?,
            limits: self.limits.build(params)?,
        };
        resources.verify()?;
        Ok(resources)
    }
}

#[derive(Deserialize, Clone, Default)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourcesSource {
    pub cpu: Option<RelaxedString>,
    pub memory: Option<RelaxedString>,
}

impl Build<ResourceRequest<String>, ()> for ResourcesSource {
    fn build(self, _: &()) -> Result<ResourceRequest<String>> {

        Ok(ResourceRequest {
            cpu: self.cpu.require("cpu")?.to_string(),
            memory: self.memory.require("cpu")?.to_string(),
        })
    }
}
