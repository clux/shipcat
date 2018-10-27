use super::structs::Resources;
use super::{Result, Manifest};

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
    pub fn estimate_wait_time(&self) -> u32 {
        // 512 default => extra 120s wait
        let pulltimeestimate = (((self.imageSize.unwrap()*120) as f64)/(1024 as f64)) as u32;
        let rcount = self.replicaCount.unwrap(); // this is set by defaults!
        // NB: we wait to pull on each node because of how rolling-upd
        if let Some(ref hc) = self.health {
            // wait for at most (bootTime + pulltimeestimate) * replicas
            (hc.wait + pulltimeestimate) * rcount
        } else if let Some(ref rp) = self.readinessProbe {
            // health equivalent for readinessProbes
            (rp.initialDelaySeconds + pulltimeestimate) * rcount
        } else {
            // sensible guess for boot time (helm default is 300 without any context)
            (30 + pulltimeestimate) * rcount
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



#[cfg(test)]
mod tests {
    use tests::setup;
    use super::super::structs::HealthCheck;
    use super::Manifest;

    #[test]
    fn wait_time_check() {
        setup();
        // DEFAULT SETUP: no values == defaults => 180s helm wait
        let mut mf = Manifest::default();
        mf.imageSize = Some(512);
        mf.health = Some(HealthCheck {
            uri: "/".into(),
            wait: 30,
            ..Default::default()
        });
        mf.replicaCount = Some(2);
        let wait = mf.estimate_wait_time();
        assert_eq!(wait, (30+60)*2);

        // setup with large image and short boot time:
        mf.imageSize = Some(4096);
        mf.health = Some(HealthCheck {
            uri: "/".into(),
            wait: 20,
            ..Default::default()
        });
        let wait2 = mf.estimate_wait_time();
        assert_eq!(wait2, (20+480)*2);
    }
}
