use crate::{ErrorKind, Manifest, Result};
use k8s_openapi::api::{
    apps::v1::{Deployment, ReplicaSet, StatefulSet},
    core::v1::Pod,
};
use kube::{
    api::{Api, DeleteParams, ListParams, LogParams, Object, ObjectList, PatchParams, Resource},
    client::APIClient,
};
use shipcat_definitions::{
    manifest::ShipcatManifest,
    status::{Applier, ManifestStatus},
};

/// Client creator
///
/// TODO: embed inside shipcat::apply when needed for other things
async fn make_client() -> Result<APIClient> {
    let config = if let Ok(cfg) = kube::config::incluster_config() {
        cfg
    } else {
        kube::config::load_kube_config()
            .await
            .map_err(ErrorKind::KubeError)?
    };
    Ok(kube::client::APIClient::new(config))
}
#[derive(Clone, Serialize, Deserialize)]
pub struct MinimalManifest {
    pub name: String,
    pub version: String,
}
type MinimalMfCrd = Object<MinimalManifest, ManifestStatus>;

/// Interface for dealing with kubernetes shipcatmanifests
pub struct ShipKube {
    mfs: Resource,
    client: APIClient,
    pub(crate) applier: Applier,
    api: Api<ShipcatManifest>,
    name: String,
    namespace: String,
}

/// Entry points for shipcat::apply, and shipcat::status
impl ShipKube {
    pub async fn new_within(svc: &str, ns: &str) -> Result<Self> {
        // hide the client in here -> Api resource for now (not needed elsewhere)
        let client = make_client().await?;
        let mfs = Resource::namespaced::<ShipcatManifest>(ns);
        let api = Api::namespaced(client.clone(), ns);

        Ok(Self {
            name: svc.to_string(),
            namespace: ns.to_string(),
            applier: Applier::infer(),
            api,
            client,
            mfs,
        })
    }

    pub async fn new(mf: &Manifest) -> Result<Self> {
        Self::new_within(&mf.name, &mf.namespace).await
    }

    /// Apply a Manifest (e.g. it's CRD wrapper)
    pub async fn apply(&self, mf: Manifest) -> Result<bool> {
        assert!(mf.version.is_some()); // ensure crd is in right state w/o secrets
        assert!(mf.is_base());
        // Wrap in the Crd Struct:
        let svc = mf.name.clone();
        let ns = mf.namespace.clone();
        let mfcrd = ShipcatManifest::new(&svc, mf);
        // TODO: use server side apply in 1.15
        // for now, shell out to kubectl
        use crate::kubectl;
        kubectl::apply_resource(&svc, mfcrd, &ns).await
    }

    /// Full CRD fetcher
    pub async fn get(&self) -> Result<ShipcatManifest> {
        let o = self.api.get(&self.name).await.map_err(ErrorKind::KubeError)?;
        Ok(o)
    }

    /// Minimal CRD fetcher (for upgrades)
    pub async fn get_minimal(&self) -> Result<MinimalMfCrd> {
        let req = self.mfs.get(&self.name).map_err(ErrorKind::KubeError)?;
        let o = self
            .client
            .request::<MinimalMfCrd>(req)
            .await
            .map_err(ErrorKind::KubeError)?;
        Ok(o)
    }

    /// Minimal CRD deleter
    pub async fn delete(&self) -> Result<()> {
        let dp = DeleteParams::default();
        let req = self.mfs.delete(&self.name, &dp).map_err(ErrorKind::KubeError)?;
        self.client
            .request::<MinimalManifest>(req)
            .await
            .map_err(ErrorKind::KubeError)?;
        Ok(())
    }

    // helper to send a merge patch
    pub async fn patch(&self, data: &serde_json::Value) -> Result<()> {
        let pp = PatchParams::default();
        // Run this patch with a smaller deserialization surface via kube::Resource
        // kube::Api would force ShipcatManifest fully valid here
        // and this would prevent status updates during schema changes.
        let req = self
            .mfs
            .patch_status(&self.name, &pp, serde_json::to_vec(data)?)
            .map_err(ErrorKind::KubeError)?;
        let o = self
            .client
            .request::<MinimalMfCrd>(req) // <- difference from using Api::patch_status
            .await
            .map_err(ErrorKind::KubeError)?;
        debug!("Patched status: {:?}", o.status);
        Ok(())
    }

    // helper to get pod data
    pub async fn get_pods(&self) -> Result<ObjectList<Pod>> {
        let api: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let lp = ListParams {
            label_selector: Some(format!("app={}", self.name)),
            ..Default::default()
        };
        let pods = api.list(&lp).await.map_err(ErrorKind::KubeError)?;
        Ok(pods)
    }

    // helper to get pods by pod hash
    pub async fn get_pods_by_template_hash(&self, hash: &str) -> Result<ObjectList<Pod>> {
        let api: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let lp = ListParams {
            label_selector: Some(format!("app={},pod-template-hash={}", self.name, hash)),
            ..Default::default()
        };
        let pods = api.list(&lp).await.map_err(ErrorKind::KubeError)?;
        Ok(pods)
    }

    // helper to get pod logs
    pub async fn get_pod_logs(&self, podname: &str) -> Result<String> {
        let api: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let lp = LogParams {
            tail_lines: Some(30),
            container: Some(self.name.to_string()),
            ..Default::default()
        };
        let logs = api.logs(podname, &lp).await.map_err(ErrorKind::KubeError)?;
        Ok(logs)
    }

    // helper to get rs data
    pub async fn get_rs(&self) -> Result<ObjectList<ReplicaSet>> {
        let api: Api<ReplicaSet> = Api::namespaced(self.client.clone(), &self.namespace);
        let lp = ListParams {
            label_selector: Some(format!("app={}", self.name)),
            ..Default::default()
        };
        let rs = api.list(&lp).await.map_err(ErrorKind::KubeError)?;
        Ok(rs)
    }

    // helper to get rs by template hash
    pub async fn get_rs_by_template_hash(&self, hash: &str) -> Result<Option<ReplicaSet>> {
        let api: Api<ReplicaSet> = Api::namespaced(self.client.clone(), &self.namespace);
        let lp = ListParams {
            label_selector: Some(format!("app={},pod-template-hash={}", self.name, hash)),
            ..Default::default()
        };
        let rs = api.list(&lp).await.map_err(ErrorKind::KubeError)?;
        Ok(rs.items.first().map(Clone::clone))
    }

    // helper to get rs from deployment
    pub async fn get_rs_from_deploy(&self) -> Result<Option<ReplicaSet>> {
        let deps: Api<Deployment> = Api::namespaced(self.client.clone(), &self.namespace);
        let replicasets: Api<ReplicaSet> = Api::namespaced(self.client.clone(), &self.namespace);

        // Get owning deployment and its revision annotation
        let dep = deps.get(&self.name).await.map_err(ErrorKind::KubeError)?;
        let mut rev = None;
        if let Some(meta) = dep.metadata {
            if let Some(annot) = meta.annotations {
                if let Some(r) = annot.get("deployment.kubernetes.io/revision") {
                    rev = Some(r.clone());
                    debug!("Desired deployment revision for {} is {}", self.name, r);
                }
            }
        }

        // If that worked, match it up to a replicaset:
        if let Some(desired) = rev {
            // Find all replicasets with our app label
            let lp = ListParams {
                label_selector: Some(format!("app={}", self.name)),
                ..Default::default()
            };
            let rs = replicasets.list(&lp).await.map_err(ErrorKind::KubeError)?;

            // Rely on kubernetes' annotation conventions
            let matching = rs
                .iter()
                .find(|r| {
                    if let Some(meta) = &r.metadata {
                        if let Some(annot) = &meta.annotations {
                            if let Some(found) = annot.get("deployment.kubernetes.io/revision") {
                                if found == &desired {
                                    debug!("Tracking replicaset revision {} for {}", found, self.name);
                                    return true;
                                }
                            }
                        }
                    }
                    false
                })
                .map(Clone::clone);
            Ok(matching)
        } else {
            // If matching up fails, we __could__ try to get the latest one...
            // But this was problematic in the case of a perfect rollback:
            // Kubernetes might re-use an existing replicaset in this case,
            // causing us to return a more recent, but downscaling replicaset.
            Ok(None)
        }
    }

    // helper to get deployment data
    pub async fn get_deploy(&self) -> Result<Deployment> {
        let api: Api<Deployment> = Api::namespaced(self.client.clone(), &self.namespace);
        let deps = api.get(&self.name).await.map_err(ErrorKind::KubeError)?;
        Ok(deps)
    }

    // helper to get statefulset data
    pub async fn get_statefulset(&self) -> Result<StatefulSet> {
        let api: Api<StatefulSet> = Api::namespaced(self.client.clone(), &self.namespace);
        let ssets = api.get(&self.name).await.map_err(ErrorKind::KubeError)?;
        Ok(ssets)
    }
}
