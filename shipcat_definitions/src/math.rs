use super::structs::Resources;
use super::structs::rollingupdate::{RollingUpdate};
use super::{Result, Manifest};

/// Total resource usage for a Manifest
///
/// Accounting for workers, replicas, sidecars, and autoscaling policies for these.
#[derive(Serialize, Default)]
pub struct ResourceTotals {
    /// Sum of basic resource structs (ignoring autoscaling limits)
    pub base: Resources<f64>,
    /// Autoscaling Ceilings on top of required
    pub extra: Resources<f64>,
}

/// Calculations done based on values in manifests
///
/// These generally assume that `verify` has passed on all manifests.
impl Manifest {

    /// Estimate how long to wait for a kube rolling upgrade
    ///
    /// Was used by helm, now used by the internal upgrade wait time.
    pub fn estimate_wait_time(&self) -> Option<u32> {
        if let (Some(size), Some(rcount)) = (self.imageSize, self.replicaCount) {
            // 512 default => extra 120s wait
            let pulltimeestimate = (((size*120) as f64)/(1024 as f64)) as u32;

            let rollout_iterations = if let Some(ru) = self.rollingUpdate {
                ru.rollout_iterations(rcount)
            } else {
                RollingUpdate::rollout_iterations_default(rcount)
            };

            // TODO: handle install case, factor of 5?

            // NB: we wait to pull on each node because of how rolling-upd
            if let Some(ref hc) = self.health {
                // wait for at most (bootTime + pulltimeestimate) * replicas
                Some((hc.wait + pulltimeestimate) * rollout_iterations)
            } else if let Some(ref rp) = self.readinessProbe {
                // health equivalent for readinessProbes
                Some((rp.initialDelaySeconds + pulltimeestimate) * rollout_iterations)
            } else {
                // sensible guess for boot time (helm default is 300 without any context)
                Some((30 + pulltimeestimate) * rollout_iterations)
            }
        } else {
            warn!("Missing imageSize or replicaCount in {}", self.name);
            None
        }

    }

    /// Compute the total resource usage of a service
    ///
    /// This relies on the `Mul` and `Add` implementations of `Resources<f64>`,
    /// which allows us to do `+` and `*` on a normalised Resources struct.
    pub fn compute_resource_totals(&self) -> Result<ResourceTotals> {
        let mut base : Resources<f64> = Resources::default();
        let mut extra : Resources<f64> = Resources::default(); // autoscaling limits
        let res = self.resources.clone().unwrap().normalised()?; // exists by verify
        if let Some(ref ascale) = self.autoScaling {
            base = base + (res.clone() * ascale.minReplicas);
            extra = extra + (res.clone() * (ascale.maxReplicas - ascale.minReplicas));
        }
        else if let Some(rc) = self.replicaCount {
            // can trust the replicaCount here
            base = base + (res.clone() * rc);
            for s in &self.sidecars {
                if let Some(ref scrsc) = s.resources {
                    // sidecar replicaCount == main deployment replicaCount
                    base = base + scrsc.normalised()? * rc;
                }
                // TODO: mandatory? sidecar resources when using sidecars?
            }
        } else {
            bail!("{} does not have replicaCount", self.name);
        }
        for w in &self.workers {
            base = base + (w.resources.normalised()? * w.replicaCount);
            // TODO: account for autoscaling in workers when it's there

            // NB: workers get the same sidecars!
            for s in &self.sidecars {
                if let Some(ref scrsc) = s.resources {
                    // worker sidecar replicaCount == worker deployment replicaCount
                    base = base + scrsc.normalised()? * w.replicaCount;
                }
                // TODO: mandatory? sidecar resources when using sidecars?
            }

        }
        Ok(ResourceTotals { base, extra })
    }

}
