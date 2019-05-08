use shipcat_definitions::{Result};
use shipcat_definitions::structs::resources::{ResourceRequirements, Resources};

use crate::util::{Build, RelaxedString, Require};

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
    fn build(self, params: &()) -> Result<Resources<String>> {

        Ok(Resources {
            cpu: self.cpu.require("cpu")?.build(params)?,
            memory: self.memory.require("cpu")?.build(params)?,
        })
    }
}
