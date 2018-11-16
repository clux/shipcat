use kubernetes::client::APIClient;
use std::collections::HashMap;
use std::env;
use shipcat_definitions::{Crd, CrdList, Manifest, Config};

use failure::{Error, Fail};
pub type Result<T> = std::result::Result<T, Error>;

static GROUPNAME: &str = "babylontech.co.uk";
static SHIPCATMANIFESTS: &str = "shipcatmanifests";
static SHIPCATCONFIGS: &str = "shipcatconfigs";

// Request builders
fn make_all_crd_entry_req(resource: &str, group: &str) -> Result<http::Request<Vec<u8>>> {
    let urlstr = format!("/apis/{group}/v1/namespaces/dev/{resource}?",
        group = group, resource = resource);
    let urlstr = url::form_urlencoded::Serializer::new(urlstr).finish();
    let mut req = http::Request::get(urlstr);
    req.body(vec![]).map_err(Error::from)
}
fn make_crd_entry_req(resource: &str, group: &str, name: &str) -> Result<http::Request<Vec<u8>>> {
    // TODO: namespace from evar
    let urlstr = format!("/apis/{group}/v1/namespaces/dev/{resource}/{name}?",
        group = group, resource = resource, name = name);
    let urlstr = url::form_urlencoded::Serializer::new(urlstr).finish();
    let mut req = http::Request::get(urlstr);
    req.body(vec![]).map_err(Error::from)
}
/*fn watch_crd_entry_after(resource: &str, group: &str, name: &str, rver: u32) -> Result<http::Request<Vec<u8>>> {
    let urlstr = format!("/apis/{group}/v1/namespaces/dev/{resource}/{name}?",
        group = group, resource = resource, name = name);
    let mut qp = url::form_urlencoded::Serializer::new(urlstr);

    qp.append_pair("timeoutSeconds", "30");
    qp.append_pair("watch", "true");

    // last version to watch after
    //qp.append_pair("resourceVersion", &rver.to_string());

    let urlstr = qp.finish();
    let mut req = http::Request::get(urlstr);
    req.body(vec![]).map_err(Error::from)
}*/


// program interface - request consumers
pub type ManifestMap = HashMap<String, Crd<Manifest>>;

pub fn get_shipcat_manifests(client: &APIClient) -> Result<ManifestMap> {
    let req = make_all_crd_entry_req(SHIPCATMANIFESTS, GROUPNAME)?;
    let res = client.request::<CrdList<Manifest>>(req)?;
    let mut data = HashMap::new();
    for i in res.items {
        data.insert(i.spec.name.clone(), i);
    }
    let keys = data.keys().cloned().into_iter().collect::<Vec<_>>().join(", ");
    debug!("Initialized with: {}", keys);
    Ok(data)
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

// version fetching stuff
#[derive(Deserialize)]
struct Entry {
    //container: String,
    name: String,
    version: String,
}


// The actual HTTP GET logic
pub fn get_version(svc: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let vurl = "https://services-uk.dev.babylontech.co.uk/status/version";
    // TODO: if in-cluster can use "version";
    let mut res = client.get(vurl).send()?;
    if !res.status().is_success() {
        debug!("failed to get version");
        bail!("Failed to fetch version");
    }
    let text = res.text()?;
    debug!("Got version data: {}", text);
    let data : Vec<Entry> = serde_json::from_str(&text)?;
    if let Some(entry) = data.into_iter().find(|r| r.name == svc) {
        Ok(entry.version)
    } else {
        bail!("No version found in version endpoint")
    }
}

// version fetching stuff
#[derive(Deserialize)]
struct Project {
    slug: String,
    name: String,
}

// Get Sentry info!
pub fn get_sentry_slug(sentry_url: &str, env: &str, svc: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let token = match env::var("SENTRY_TOKEN") {
        Ok(val) => val,
        Err(e)  => bail!("SENTRY_TOKEN env var not found"),
    };

    let projects_url = format!("{sentry_url}/api/0/teams/sentry/{env}/projects/",
                               sentry_url = &sentry_url,
                               env = &env);

    let mut res = client
        .get(reqwest::Url::parse(&projects_url).unwrap())
        .header("Authorization", format!("Bearer {token}", token = token))
        .send()?;
    if !res.status().is_success() {
        debug!("failed to get projects");
        bail!("Failed to fetch projects in team {}", env);
    }
    let text = res.text()?;
    debug!("Got data: {}", text);
    let data : Vec<Project> = serde_json::from_str(&text)?;
    if let Some(entry) = data.into_iter().find(|r| r.name == svc) {
        Ok(entry.slug)
    } else {
        bail!("Project {} not found in team {}, {}", svc, env, sentry_url)
    }
}
