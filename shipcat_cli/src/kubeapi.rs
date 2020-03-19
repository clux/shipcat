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
    pub async fn patch(&self, data: &serde_json::Value) -> Result<ShipcatManifest> {
        let pp = PatchParams::default();
        let o = self
            .api
            .patch_status(&self.name, &pp, serde_json::to_vec(data)?)
            .await
            .map_err(ErrorKind::KubeError)?;
        debug!("Patched status: {:?}", o.status);
        Ok(o)
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

    // helper to get the latest rs
    pub async fn get_rs_latest(&self) -> Result<Option<ReplicaSet>> {
        let api: Api<ReplicaSet> = Api::namespaced(self.client.clone(), &self.namespace);
        let lp = ListParams {
            label_selector: Some(format!("app={}", self.name)),
            ..Default::default()
        };
        let rs = api.list(&lp).await.map_err(ErrorKind::KubeError)?;
        let mut rssorted = rs.into_iter().collect::<Vec<ReplicaSet>>();

        rssorted.sort_by_key(|rs| {
            // TODO: creation(&rs).unwrap_or_else(|| Time(Utc::now())
            // awaiting k8s-opinapi version
            rs.metadata
                .as_ref()
                .expect("rs has metadata")
                .creation_timestamp
                .as_ref()
                .expect("rs has creation timestamp")
                .0
        });
        Ok(rssorted.last().map(Clone::clone))
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

/*
// For above TODO ^
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
fn creation(rs: &ReplicaSet) -> Option<Time> {
    if let Some(meta) = &rs.metadata {
        if let Some(ts) = &meta.creation_timestamp {
            return Some(ts.clone())
        }
        warn!("No creation timestamp for replicaset {:?}", meta.name);
        return None;
    }
    warn!("No creation timestamp for replicaset");
    return None;
}*/
