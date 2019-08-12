use merge::Merge;

use shipcat_definitions::Result;
use shipcat_definitions::structs::Worker;
use shipcat_definitions::structs::autoscaling::AutoScaling;

use crate::util::{Build, Require, RelaxedString};
use std::collections::BTreeMap;
use super::source::{ContainerSource, ContainerBuildParams};

#[derive(Deserialize, Merge, Clone, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkerSource {
    pub replica_count: Option<u32>,
    pub auto_scaling: Option<AutoScaling>,
    pub http_port: Option<u32>,
    pub pod_annotations: BTreeMap<String, RelaxedString>,

    #[serde(flatten)]
    pub container: ContainerSource,
}

impl Build<Worker, ContainerBuildParams> for WorkerSource {
    fn build(self, params: &ContainerBuildParams) -> Result<Worker> {
        if let Some(a) = &self.auto_scaling {
            a.verify()?;
        }
        Ok(Worker {
            container: self.container.build(params)?,
            replicaCount: self.replica_count.require("replicaCount")?,
            autoScaling: self.auto_scaling,
            httpPort: self.http_port,
            podAnnotations: self.pod_annotations.build(&())?,
        })
    }
}
