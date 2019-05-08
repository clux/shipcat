use shipcat_definitions::Result;
use shipcat_definitions::structs::Container;

use crate::util::{Build, Require};
use super::container::{ContainerSource, ContainerBuildParams};

#[derive(Deserialize, Clone, Default)]
pub struct InitContainerSource(ContainerSource);

impl Build<Container, ContainerBuildParams> for InitContainerSource {
    fn build(self, params: &ContainerBuildParams) -> Result<Container> {
        let mut container = self.0.build(params)?;
        container.image = Some(container.image.require("image")?);
        Ok(container)
    }
}
