use kubernetes::client::APIClient;
use std::collections::BTreeMap;
use serde_json;
use shipcat_definitions::{Crd, CrdList, CrdEvent, Manifest, Config};

use super::{Result, Error};

static GROUPNAME: &str = "babylontech.co.uk";
static SHIPCATMANIFESTS: &str = "shipcatmanifests";
static SHIPCATCONFIGS: &str = "shipcatconfigs";
static LASTAPPLIED: &str = "kubectl.kubernetes.io/last-applied-configuration";

// Request builders
fn make_all_crd_entry_req(resource: &str, group: &str) -> Result<http::Request<Vec<u8>>> {
    let ns = std::env::var("ENV_NAME").expect("Must have an env name evar");
    let urlstr = format!("/apis/{group}/v1/namespaces/{ns}/{resource}?",
        group = group, resource = resource, ns = ns);
    let urlstr = url::form_urlencoded::Serializer::new(urlstr).finish();
    let mut req = http::Request::get(urlstr);
    req.body(vec![]).map_err(Error::from)
}
fn make_crd_entry_req(resource: &str, group: &str, name: &str) -> Result<http::Request<Vec<u8>>> {
    let ns = std::env::var("ENV_NAME").expect("Must have an env name evar");
    let urlstr = format!("/apis/{group}/v1/namespaces/{ns}/{resource}/{name}?",
        group = group, resource = resource, name = name, ns = ns);
    let urlstr = url::form_urlencoded::Serializer::new(urlstr).finish();
    let mut req = http::Request::get(urlstr);
    req.body(vec![]).map_err(Error::from)
}
fn watch_crd_entry_after(resource: &str, group: &str, ver: &str) -> Result<http::Request<Vec<u8>>> {
    //let urlstr = format!("/apis/{group}/v1/namespaces/dev/{resource}/{name}?",
    //    group = group, resource = resource, name = name);
    let urlstr = format!("/apis/{group}/v1/namespaces/dev/{resource}?",
        group = group, resource = resource);
    let mut qp = url::form_urlencoded::Serializer::new(urlstr);

    qp.append_pair("timeoutSeconds", "10");
    qp.append_pair("watch", "true");
    qp.append_pair("resourceVersion", ver);

    let urlstr = qp.finish();
    let mut req = http::Request::get(urlstr);
    req.body(vec![]).map_err(Error::from)
}


pub fn watch_for_shipcat_manifest_updates(client: &APIClient, old: ManifestCache) -> Result<ManifestMap> {
    let req = watch_crd_entry_after(SHIPCATMANIFESTS, GROUPNAME, &old.version)?;
    let res = client.request_events::<CrdEvent<Manifest>>(req)?;
    let mut data = BTreeMap::new();
    // TODO: catch gone error - and trigger a list
    //{"type":"ERROR","object":{"kind":"Status","apiVersion":"v1","metadata":{},"status":"Failure","message":"too old resource version: 185325401 (185325402)","reason":"Gone","code":410}}
    for i in res {
        let crd = i.object;
        println!("Got {:?} event for {}", i.kind, crd.spec.name);
        if let Some(last_annot) = crd.metadata.annotations.get(LASTAPPLIED) {
            let oldmf = old.manifests.get(&crd.spec.name);
            println!("comparing {} with old annotation", crd.spec.name);
            let lastmf : Crd<Manifest> = serde_json::from_str(last_annot)?;
            if serde_json::to_string(&lastmf.spec)? != serde_json::to_string(&oldmf)? {
                println!("Found to be different!");
                data.insert(crd.spec.name.clone(), crd.spec);
            }
        }
    }
    let keys = data.keys().cloned().into_iter().collect::<Vec<_>>().join(", ");
    debug!("Updated: {}", keys);
    Ok(data)
}


// program interface - request consumers
#[derive(Default, Clone)]
pub struct ManifestCache {
    pub manifests: ManifestMap,
    pub version: String, // kube keeps it as a String
}
pub type ManifestMap = BTreeMap<String, Manifest>;

pub fn get_shipcat_manifests(client: &APIClient) -> Result<ManifestCache> {
    let req = make_all_crd_entry_req(SHIPCATMANIFESTS, GROUPNAME)?;
    let res = client.request::<CrdList<Manifest>>(req)?;
    let mut manifests = BTreeMap::new();
    let version = res.metadata.resourceVersion;
    info!("Got {} at resource version: {}", res.kind, version);

    for i in res.items {
        manifests.insert(i.spec.name.clone(), i.spec);
    }
    let keys = manifests.keys().cloned().into_iter().collect::<Vec<_>>().join(", ");
    debug!("Initialized with: {}", keys);
    Ok(ManifestCache { manifests, version })
}

pub fn get_shipcat_config(client: &APIClient, name: &str) -> Result<Crd<Config>> {
    let req = make_crd_entry_req(SHIPCATCONFIGS, GROUPNAME, name)?;
    let res = client.request::<Crd<Config>>(req)?;
    debug!("got config with version {}", &res.spec.version);
    // TODO: merge with version found in rolling env?
    Ok(res)
}

/*this doesn't actually work...
pub fn watch_shipcat_manifest(client: &APIClient, name: &str, rver: u32) -> Result<Crd<Manifest>> {
    let req = watch_crd_entry_after(SHIPCATMANIFESTS, GROUPNAME, name, rver)
        .expect("failed to define crd watch request");
    let res = client.request::<Crd<_>>(req)?;
    debug!("{}", &res.spec.name);
    Ok(res)
}*/

// actually unused now because everything returns from cache
/*pub fn get_shipcat_manifest(client: &APIClient, name: &str) -> Result<Crd<Manifest>> {
    let req = make_crd_entry_req(SHIPCATMANIFESTS, GROUPNAME, name)?;
    let res = client.request::<Crd<Manifest>>(req)?;
    debug!("got {}", &res.spec.name);
    // TODO: merge with version found in rolling env?
    Ok(res)
}
*/
