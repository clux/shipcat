use merge::Merge;
use regex::Regex;

use shipcat_definitions::{
    structs::{Container, Probe, VolumeMount},
    Result,
};

use crate::util::{Build, Require};

use super::{
    image::{ImageNameSource, ImageTagSource},
    port::PortSource,
    resources::ResourceRequirementsSource,
    EnvVarsSource,
};

#[derive(Deserialize, Clone, Default)]
pub struct ContainerName(String);

impl Build<String, ()> for ContainerName {
    fn build(self, _: &()) -> Result<String> {
        let Self(name) = self;
        let re = Regex::new(r"^[0-9a-z\-]{1,50}$").unwrap();
        if !re.is_match(&name) {
            bail!("Name must be alphanumeric (with dashes) and 1-50 characters");
        }
        Ok(name)
    }
}

/// Source configuration for a K8s container, deserialized from a service manifest.
#[derive(Deserialize, Merge, Clone, Default)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ContainerSource {
    pub name: Option<ContainerName>,
    pub image: Option<ImageNameSource>,
    pub version: Option<ImageTagSource>,

    pub resources: Option<ResourceRequirementsSource>,

    pub command: Option<Vec<String>>,
    pub env: EnvVarsSource,
    pub preserve_env: Option<bool>,

    pub readiness_probe: Option<Probe>,
    pub liveness_probe: Option<Probe>,

    pub ports: Option<Vec<PortSource>>,

    pub volume_mounts: Option<Vec<VolumeMount>>,
}

pub struct ContainerBuildParams {
    pub main_envs: EnvVarsSource,
}

impl Build<Container, ContainerBuildParams> for ContainerSource {
    fn build(self, params: &ContainerBuildParams) -> Result<Container> {
        let env = if self.preserve_env.unwrap_or_default() {
            params.main_envs.clone().merge(self.env)
        } else {
            self.env
        };
        if let Some(rp) = &self.readiness_probe {
            // TODO: Inline
            rp.verify()?;
        }
        if let Some(lp) = &self.liveness_probe {
            // TODO: Inline
            lp.verify()?;
        }
        Ok(Container {
            name: self.name.require("name")?.build(&())?,
            image: self.image.build(&())?,
            version: self.version.build(&())?,

            resources: self.resources.build(&())?,

            command: self.command.unwrap_or_default(),
            env: env.build(&())?,

            readiness_probe: self.readiness_probe,
            liveness_probe: self.liveness_probe,

            ports: self.ports.unwrap_or_default().build(&())?,

            volume_mounts: self.volume_mounts.unwrap_or_default(),
        })
    }
}
