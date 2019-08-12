mod source;
pub use source::ContainerBuildParams;

mod env;
mod resources;
mod image;

pub use env::EnvVarsSource;
pub use resources::ResourceRequirementsSource;
pub use image::{ImageNameSource, ImageTagSource};

mod cronjob;
mod initcontainer;
mod job;
mod sidecar;
mod worker;

pub use cronjob::CronJobSource;
pub use initcontainer::InitContainerSource;
pub use job::JobSource;
pub use sidecar::SidecarSource;
pub use worker::WorkerSource;
