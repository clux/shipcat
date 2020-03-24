// AutoScaling types roughly as defined in kubernetes source
// https://github.com/kubernetes/kubernetes/blob/master/pkg/apis/autoscaling/types.go
// https://docs.rs/k8s-openapi/0.7.1/k8s_openapi/api/autoscaling/v2beta2/struct.HorizontalPodAutoscalerSpec.html

use super::Result;
use k8s_openapi::api::autoscaling::v2beta2::MetricSpec;

/// Configuration parameters for HorizontalPodAutoScaler
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AutoScaling {
    pub minReplicas: u32,
    pub maxReplicas: u32,

    /// Specifications to use to calculate the desired replica count
    ///
    /// The desired replica count is calculated multiplying the ratio between
    /// the target value and the current value by the current number of pods.
    /// Ergo, metrics used must decrease as the pod count is increased, and vice-versa.
    /// See the individual metric source types for more information about how each
    /// type of metric must respond.
    /// If not set, the default metric will be set to 80% average CPU utilization.
    ///
    /// The maximum replica count across all metrics will be used.
    pub metrics: Vec<MetricSpec>,
}

impl AutoScaling {
    pub fn verify(&self) -> Result<()> {
        if self.minReplicas > self.maxReplicas {
            bail!("maxReplicas must be > minReplicas");
        }
        Ok(())
    }
}
