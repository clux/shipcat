use shipcat_definitions::{Result};
use shipcat_definitions::structs::resources::{ResourceRequirements, Resources};
use shipcat_definitions::deserializers::{RelaxedString};

use crate::util::{Build, Require};

#[derive(Deserialize, Clone, Default)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceRequirementsSource {
    pub requests: ResourcesSource,
    pub limits: ResourcesSource,
}

impl Build<ResourceRequirements<String>, ()> for ResourceRequirementsSource {
    fn build(self, params: &()) -> Result<ResourceRequirements<String>> {
        let resources = ResourceRequirements {
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

impl Build<Resources<String>, ()> for ResourcesSource {
    fn build(self, _: &()) -> Result<Resources<String>> {

        Ok(Resources {
            cpu: self.cpu.require("cpu")?.to_string(),
            memory: self.memory.require("cpu")?.to_string(),
        })
    }
}
