// AutoScaling types roughly as defined in kubernetes source
// https://github.com/kubernetes/kubernetes/blob/master/pkg/apis/autoscaling/types.go

use super::{Result};

/// Configuration parameters for HorizontalPodAutoScaler
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AutoScaling {
    pub minReplicas: u32,
    pub maxReplicas: u32,

    /// Metrics to scale on
    pub metrics: Vec<ScalingMetric>,
}

/// Kubernetes Scaling Metrics (adjacently tagged enums)
///
/// The content name (for adjacency) is dynamic - so need wrapper structs..
/// The name of the wrapper is tagged correctly via serde under a `type` key
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum ScalingMetric {
    Resource(ScalingMetricResourceWrapper),
    Pods(ScalingMetricPodWrapper),
    // no Object support yet
}

// dumb adjacency wrappers to get the adjacency content
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScalingMetricResourceWrapper { resource: ScalingMetricResource }
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScalingMetricPodWrapper { pods: ScalingMetricPod }

/// Native resource scaling via kube
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScalingMetricResource {
    name: ScalingMetricResourceType,
    /// The target value of the average of the resource metric across relevant pods,
    /// represented as a percentage of the requested value of the resource for the pods
    #[serde(default, skip_serializing_if = "Option::is_none")]
    targetAverageUtilization: Option<u32>,
    /// The target value of the average of the resource metric across relevant pods,
    /// represented as a raw value (not percentage of request)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    targetAverageValue: Option<String>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ScalingMetricResourceType {
    #[serde(rename = "cpu")]
    CPU,
    #[serde(rename = "memory")]
    Memory,
}

/// Scaling Metrics from prometheus
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScalingMetricPod {
    /// Promethus metric name
    pub metricName: String,
    pub targetAverageValue: String,
}

impl AutoScaling {
     pub fn verify(&self) -> Result<()> {
        if self.minReplicas == 0 {
            bail!("minReplicas must be at least 1");
        }
        if self.minReplicas > self.maxReplicas {
            bail!("maxReplicas must be > minReplicas");
        }
        for m in &self.metrics {
            match m {
                ScalingMetric::Resource(r) => {
                    // need at least one of the values
                    // docs seem to hint at one for cpu and one for memory...
                    // if this is the case should disallow both to be set..
                    match r.resource.name {
                        ScalingMetricResourceType::CPU => {
                            assert!(r.resource.targetAverageUtilization.is_some());
                        },
                        ScalingMetricResourceType::Memory => {
                            assert!(r.resource.targetAverageValue.is_some());
                        }
                    }
                },
                ScalingMetric::Pods(_p) => {} // no validation here
            }
        }

        Ok(())
     }
}
