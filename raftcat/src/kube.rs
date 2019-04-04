use kubernetes::client::APIClient;
use std::collections::BTreeMap;
use shipcat_definitions::{Crd, CrdList, CrdEvent, CrdEventType, Manifest, Config};

use super::{Result, Error};

static GROUPNAME: &str = "babylontech.co.uk";
static SHIPCATMANIFESTS: &str = "shipcatmanifests";
static SHIPCATCONFIGS: &str = "shipcatconfigs";
//static LASTAPPLIED: &str = "kubectl.kubernetes.io/last-applied-configuration";

struct ResourceGroup {
    /// API Group
    group: String,
    /// API Resource
    resource: String,
    /// Namespace the resources resides
    namespace: String,
}
impl ResourceGroup {
    pub fn config(namespace: &str) -> Self {
        ResourceGroup {
            group: GROUPNAME.to_string(),
            resource: SHIPCATCONFIGS.to_string(),
            namespace: namespace.to_string()
        }
    }
    pub fn manifest(namespace: &str) -> Self {
        ResourceGroup {
            group: GROUPNAME.to_string(),
            resource: SHIPCATMANIFESTS.to_string(),
            namespace: namespace.to_string()
        }
    }
}

// Request builders
fn make_all_crd_entry_req(r: &ResourceGroup) -> Result<http::Request<Vec<u8>>> {
    let urlstr = format!("/apis/{group}/v1/namespaces/{ns}/{resource}?",
        group = r.group, resource = r.resource, ns = r.namespace);
    let urlstr = url::form_urlencoded::Serializer::new(urlstr).finish();
    let mut req = http::Request::get(urlstr);
    req.body(vec![]).map_err(Error::from)
}
fn make_crd_entry_req(r: &ResourceGroup, name: &str) -> Result<http::Request<Vec<u8>>> {
    let urlstr = format!("/apis/{group}/v1/namespaces/{ns}/{resource}/{name}?",
        group = r.group, resource = r.resource, ns = r.namespace, name = name);
    let urlstr = url::form_urlencoded::Serializer::new(urlstr).finish();
    let mut req = http::Request::get(urlstr);
    req.body(vec![]).map_err(Error::from)
}
fn watch_crd_entry_after(r: &ResourceGroup, ver: &str) -> Result<http::Request<Vec<u8>>> {
    //let urlstr = format!("/apis/{group}/v1/namespaces/{ns}/{resource}/{name}?",
    //    group = r.group, resource = r.resource, ns = r.namespace, name = name);
    let urlstr = format!("/apis/{group}/v1/namespaces/{ns}/{resource}?",
        group = r.group, resource = r.resource, ns = r.namespace);
    let mut qp = url::form_urlencoded::Serializer::new(urlstr);

    qp.append_pair("timeoutSeconds", "10");
    qp.append_pair("watch", "true");
    qp.append_pair("resourceVersion", ver);

    let urlstr = qp.finish();
    let mut req = http::Request::get(urlstr);
    req.body(vec![]).map_err(Error::from)
}


pub fn watch_for_shipcat_manifest_updates(client: &APIClient, ns: &str, mut data: ManifestCache) -> Result<ManifestCache> {
    let rg = ResourceGroup::manifest(&ns);
    let req = watch_crd_entry_after(&rg, &data.version)?;
    let res = client.request_events::<CrdEvent<Manifest>>(req)?;
    //let mut found = vec![];
    // TODO: catch gone error - and trigger a list
    //{"type":"ERROR","object":{"kind":"Status","apiVersion":"v1","metadata":{},"status":"Failure","message":"too old resource version: 185325401 (185325402)","reason":"Gone","code":410}}
    for i in res {
        let crd = i.object;
        debug!("Got {:?} event for {} (ver={})", i.kind, crd.spec.name, crd.metadata.resourceVersion);
        // TODO: diff properly
        //if let Some(last_annot) = crd.metadata.annotations.get(LASTAPPLIED) {
        //    let oldmf = data.manifests.get(&crd.spec.name);
        //    println!("comparing {} with old annotation", crd.spec.name);
        //    let lastmf : Crd<Manifest> = serde_json::from_str(last_annot)?;
        //    if serde_json::to_string(&lastmf.spec)? != serde_json::to_string(&oldmf)? {
        //        println!("Found to be different!");
        //        found.push(crd.spec.name.clone());
        //    }
        //}
        match i.kind {
            CrdEventType::Added => {
                info!("Adding service {}", crd.spec.name);
                data.manifests.entry(crd.spec.name.clone())
                    .or_insert_with(|| crd.spec.clone());
            },
            CrdEventType::Modified => {
                info!("Modifying service {}", crd.spec.name);
                data.manifests.entry(crd.spec.name.clone())
                    .and_modify(|e| *e = crd.spec.clone());
            },
            CrdEventType::Deleted => {
                info!("Removing service {}", crd.spec.name);
                data.manifests.remove(&crd.spec.name);
            }
        }
        if crd.metadata.resourceVersion != "" {
            data.version = crd.metadata.resourceVersion.clone();
        }
    }
    //debug!("Updated: {}", found.join(", "));
    Ok(data) // updated in place (taken ownership)
}


// program interface - request consumers
#[derive(Default, Clone)]
pub struct ManifestCache {
    pub manifests: ManifestMap,
    pub version: String, // kube keeps it as a String
}
pub type ManifestMap = BTreeMap<String, Manifest>;

pub fn get_shipcat_manifests(client: &APIClient, namespace: &str) -> Result<ManifestCache> {
    let rg = ResourceGroup::manifest(namespace);
    let req = make_all_crd_entry_req(&rg)?;
    let res = client.request::<CrdList<Manifest>>(req)?;
    let mut manifests = BTreeMap::new();
    let version = res.metadata.resourceVersion;
    info!("Got {} at resource version: {}", res.kind, version);

    for i in res.items {
        manifests.insert(i.spec.name.clone(), i.spec);
    }
    let keys = manifests.keys().cloned().collect::<Vec<_>>().join(", ");
    debug!("Initialized with: {}", keys);
    Ok(ManifestCache { manifests, version })
}

pub fn get_shipcat_config(client: &APIClient, namespace: &str, name: &str) -> Result<Crd<Config>> {
    let rg = ResourceGroup::config(&namespace);
    let req = make_crd_entry_req(&rg, name)?;
    let res = client.request::<Crd<Config>>(req)?;
    debug!("got config with versions {:?}", &res.spec.versions);
    // TODO: merge with version found in rolling env?
    Ok(res)
}
