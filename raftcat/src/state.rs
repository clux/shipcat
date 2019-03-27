use log::{info, warn, error, debug, trace};
use failure::err_msg;
use tera::compile_templates;

use kubernetes::{
    client::APIClient,
    config::Configuration,
};

use std::{
    collections::BTreeMap,
    env,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use crate::*;
use crate::integrations::{
    newrelic::{self, RelicMap},
    sentryapi::{self, SentryMap},
    version,
};

/// Encapsulated data object that is contains cloneable state
///
/// This object is updated from CRD state and third party integrations.
/// Data herein should be considered canonical.
///
/// Only this file should have a write handler to this struct.
#[derive(Clone)]
struct DataState {
    pub cache: ManifestCache,
    pub config: Config,
    pub relics: RelicMap,
    pub sentries: SentryMap,
    region: String,
    last_update: Instant,
}
impl DataState {
    pub fn new(client: &APIClient) -> Result<Self> {
        info!("Loading state from CRDs");
        let rname = env::var("REGION_NAME").expect("Need REGION_NAME evar");
        let ns = env::var("NAMESPACE").expect("Need NAMESPACE evar");
        debug!("Initializing cache for {} in {}", rname, ns);
        let state = DataState::init_cache(client, &ns)?;
        let config = kube::get_shipcat_config(client, &ns, &rname)?.spec;
        let mut res = DataState {
            cache: state,
            config,
            region: rname,
            relics: BTreeMap::new(),
            sentries: BTreeMap::new(),
            last_update: Instant::now(),
        };
        res.update_slow_cache()?;
        Ok(res)
    }

    // Helper for init
    fn init_cache(client: &APIClient, ns: &str) -> Result<ManifestCache> {
        info!("Initialising state from CRDs");
        let mut data = kube::get_shipcat_manifests(client, ns)?;
        match version::get_all() {
            Ok(versions) => {
                info!("Loaded {} versions", versions.len());
                for (k, mf) in &mut data.manifests {
                    mf.version = versions.get(k).map(String::clone);
                }
            }
            Err(e) => warn!("Unable to load versions: {}", err_msg(e))
        }
        Ok(data)
    }
    fn update_slow_cache(&mut self) -> Result<()> {
        let region = self.config.get_region(&self.region).unwrap();
        if let Some(s) = region.sentry {
            match sentryapi::get_slugs(&s.url, &region.environment.to_string()) {
                Ok(res) => {
                    self.sentries = res;
                    info!("Loaded {} sentry slugs", self.sentries.len());
                },
                Err(e) => warn!("Unable to load sentry slugs: {}", err_msg(e)),
            }
        } else {
            warn!("No sentry url configured for this region");
        }
        match newrelic::get_links(&region.name) {
            Ok(res) => {
                self.relics = res;
                info!("Loaded {} newrelic links", self.relics.len());
            },
            Err(e) => warn!("Unable to load newrelic projects. {}", err_msg(e)),
        }
        Ok(())
    }
}

/// Sync state that can be shared with an actix app
///
/// Consumers of these (http handlers) should use public impls on this struct only.
/// Callers should not need to care about getting read/write locks
///
/// Anything on this list should be complicated structures that do not implement Clone.
#[derive(Clone)]
pub struct State {
    /// Templates via tera which do not implement clone
    template: Arc<RwLock<tera::Tera>>,
    /// Application state via CRDs which is synced in a separate thread
    data: Arc<RwLock<DataState>>,

    client: APIClient,
    namespace: String,
    region: String,
}

/// Wrappers around synchronised data
///
/// Note that these functions unwrap a lot and expect errors to just be caught by sentry.
/// The reason we don't return results here is that they are used directly by actix handlers
/// and as such need to be Send:
///
/// Send not implemented for std::sync::PoisonError<std::sync::RwLockReadGuard<'_, T>>
///
/// This is fine; a bad unwrap here or in a handler results in a 500 + a sentry event.
impl State {
    fn new(client: APIClient) -> Result<Self> {
        let t = compile_templates!(concat!("raftcat", "/templates/*"));
        let state = DataState::new(&client)?;
        let namespace = env::var("NAMESPACE").expect("Need NAMESPACE evar");
        let region = env::var("REGION_NAME").expect("Need REGION_NAME evar");
        Ok(State {
            client,
            namespace, region,
            data: Arc::new(RwLock::new(state)),
            template: Arc::new(RwLock::new(t)),
        })
    }
    // Template getters for main
    pub fn render_template(&self, tpl: &str, ctx: tera::Context) -> String {
        let t = self.template.read().unwrap();
        t.render(tpl, &ctx).unwrap()
    }
    // Data getters for main
    pub fn get_region(&self) -> Result<Region> {
        let data = self.data.read().unwrap();
        let r = data.config.get_region(&data.region).expect(&format!("could not resolve cluster for {}", data.region));
        Ok(r)
    }
    pub fn get_config(&self) -> Result<Config> {
        Ok(self.data.read().unwrap().config.clone())
    }
    pub fn get_manifests(&self) -> Result<ManifestMap> {
        Ok(self.data.read().unwrap().cache.manifests.clone())
    }
    pub fn get_manifest(&self, key: &str) -> Result<Option<Manifest>> {
        if let Some(mf) = self.data.read().unwrap().cache.manifests.get(key) {
            return Ok(Some(mf.clone()));
        }
        Ok(None)
    }
    pub fn get_manifests_for(&self, team: &str) -> Result<Vec<String>> {
        let mfs = self.data.read().unwrap().cache.manifests.iter()
            .filter(|(_k, mf)| mf.metadata.clone().unwrap().team == team)
            .map(|(_k, mf)| mf.name.clone()).collect();
        Ok(mfs)
    }
    pub fn get_reverse_deps(&self, service: &str) -> Result<Vec<String>> {
        let mut res = vec![];
        for (svc, mf) in &self.data.read().unwrap().cache.manifests {
            if mf.dependencies.iter().any(|d| d.name == service) {
                res.push(svc.clone())
            }
        }
        Ok(res)
    }
    pub fn get_newrelic_link(&self, service: &str) -> Option<String> {
        self.data.read().unwrap().relics.get(service).map(String::to_owned)
    }
    pub fn get_sentry_slug(&self, service: &str) -> Option<String> {
        self.data.read().unwrap().sentries.get(service).map(String::to_owned)
    }


    // state updaters from this file only
    fn full_refresh(&self) -> Result<()> {
        let res = kube::get_shipcat_manifests(&self.client, &self.namespace)?;
        let cfg = kube::get_shipcat_config(&self.client, &self.namespace, &self.region)?.spec;
        self.data.write().unwrap().cache = res;
        self.data.write().unwrap().config = cfg;
        Ok(())
    }
    fn watch_manifests(&self) -> Result<()> {
        let old = self.data.read().unwrap().cache.clone();
        let res = kube::watch_for_shipcat_manifest_updates(
            &self.client,
            &self.namespace,
            old
        )?;
        // lock to update cache
        self.data.write().unwrap().cache = res;
        Ok(())
    }
}

/// Initiailize state machine for an actix app
///
/// Returns a Sync
pub fn init(cfg: Configuration) -> Result<State> {
    let client = APIClient::new(cfg);
    let state = State::new(client)?;
    let state2 = state.clone();
    // continuously poll for updates
    use std::thread;
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(10));
            match state2.watch_manifests() {
                Ok(_) => trace!("State refreshed"),
                Err(e) => {
                    // if resourceVersions get desynced, this can happen
                    warn!("Failed to refresh {}", e);
                    // try a full refresh in a bit
                    thread::sleep(Duration::from_secs(10));
                    match state2.full_refresh() {
                        Ok(_) => info!("Full state refresh fallback succeeded"),
                        Err(e) => {
                            error!("Failed to refesh cache on fallback: '{}' - rebooting", e);
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
    });
    Ok(state)
}
