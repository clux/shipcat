use merge::Merge;

use shipcat_definitions::Result;
use shipcat_definitions::structs::CronJob;
use shipcat_definitions::structs::job::{JobVolumeClaim};

use crate::util::{Build, Require, RelaxedString};
use std::collections::BTreeMap;

use super::container::{ContainerSource, ContainerBuildParams};

#[derive(Deserialize, Merge, Clone, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct CronJobSource {
    pub schedule: Option<String>,
    pub volume_claim: Option<JobVolumeClaim>,
    pub timeout: Option<u32>,
    pub backoff_limit: Option<u16>,
    pub pod_annotations: BTreeMap<String, RelaxedString>,

    #[serde(flatten)]
    pub container: ContainerSource,
}

impl Build<CronJob, ContainerBuildParams> for CronJobSource {
    fn build(self, params: &ContainerBuildParams) -> Result<CronJob> {
        let container = self.container.build(params)?;
        match (&container.image, &container.version) {
            (Some(_), None) => bail!("Cannot specify image without specifying version in CronJob"),
            (None, Some(_)) => bail!("Cannot specify the version without specifying an image in CronJob"),
            (_, _) => (),
        };
        Ok(CronJob {
            container,
            schedule: self.schedule.require("schedule")?,
            volumeClaim: self.volume_claim,
            timeout: self.timeout,
            backoffLimit: self.backoff_limit,
            podAnnotations: self.pod_annotations.build(&())?,
        })
    }
}
