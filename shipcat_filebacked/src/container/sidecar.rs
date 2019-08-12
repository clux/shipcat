use shipcat_definitions::Result;
use shipcat_definitions::structs::Container;

use crate::util::Build;
use super::source::{ContainerSource, ContainerBuildParams};

#[derive(Deserialize, Clone, Default)]
pub struct SidecarSource(ContainerSource);

impl Build<Container, ContainerBuildParams> for SidecarSource {
    fn build(self, params: &ContainerBuildParams) -> Result<Container> {
        self.0.build(params)
    }
}
