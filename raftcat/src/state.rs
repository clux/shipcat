use failure::err_msg;
use tera::compile_templates;
use kube::{
    client::APIClient,
    config::Configuration,
    api::{Reflector, Api, Void, Object},
};

use std::{
    collections::BTreeMap,
    env,
    sync::{Arc, RwLock},
    time::Duration,
};

use crate::*;
use crate::integrations::{
    newrelic::{self, RelicMap},
    sentryapi::{self, SentryMap},
};

type ManifestObject = Object<Manifest, ManifestStatus>;
type ConfigObject = Object<Config, Void>;

/// Map of service -> versions
pub type VersionMap = BTreeMap<String, String>;

/// The canonical shared state for actix
///
/// Consumers of these (http handlers) should use public impls on this struct only.
/// Callers should not need to care about getting read/write locks.
/// Only this file should have a write handler to this struct.
#[derive(Clone)]
pub struct State {
    manifests: Reflector<ManifestObject>,
    configs: Reflector<ConfigObject>,
    relics: RelicMap,
    sentries: SentryMap,
    /// Templates via tera which do not implement clone
    template: Arc<RwLock<tera::Tera>>,
    region: String,
    config_name: String,
}

/// Note that these functions unwrap a lot and expect errors to just be caught by sentry.
/// The reason we don't return results here is that they are used directly by actix handlers
/// and as such need to be Send:
///
/// Send not implemented for std::sync::PoisonError<std::sync::RwLockReadGuard<'_, T>>
///
/// This is fine; a bad unwrap here or in a handler results in a 500 + a sentry event.
impl State {
    pub fn new(client: APIClient) -> Result<Self> {
        info!("Loading state from CRDs");
        let region = env::var("REGION_NAME").expect("Need REGION_NAME evar");
        let ns = env::var("NAMESPACE").expect("Need NAMESPACE evar");
        let t = compile_templates!(concat!("raftcat", "/templates/*"));
        debug!("Initializing cache for {} in {}", region, ns);

        let mfresource = Api::customResource(client.clone(), "shipcatmanifests")
            .version("v1")
            .group("babylontech.co.uk")
            .within(&ns);
        let cfgresource = Api::customResource(client.clone(), "shipcatconfigs")
            .version("v1")
            .group("babylontech.co.uk")
            .within(&ns);
        let manifests = Reflector::new(mfresource).init()?;
        let configs = Reflector::new(cfgresource).init()?;
        // Use federated config if available:
        let config_name = if configs.read()?.iter()
            .any(|crd : &ConfigObject| crd.metadata.name == "unionised") {
            "unionised".into()
        } else {
            region.clone()
        };
        let mut res = State {
            manifests, configs, region, config_name,
            relics: BTreeMap::new(),
            sentries: BTreeMap::new(),
            template: Arc::new(RwLock::new(t)),
        };
        res.update_slow_cache()?;
        Ok(res)
    }
    /// Template getter for main
    pub fn render_template(&self, tpl: &str, ctx: tera::Context) -> String {
        let t = self.template.read().unwrap();
        t.render(tpl, &ctx).unwrap()
    }
    // Getters for main
    pub fn get_manifests(&self) -> Result<BTreeMap<String, Manifest>> {
        let xs = self.manifests.read()?.into_iter().fold(BTreeMap::new(), |mut acc, crd| {
            acc.insert(crd.spec.name.clone(), crd.spec); // don't expose crd metadata + status
            acc
        });
        Ok(xs)
    }
    pub fn get_config(&self) -> Result<Config> {
        let cfgs = self.configs.read()?;
        if let Some(cfg) = cfgs.iter().find(|c| c.metadata.name == self.config_name) {
            Ok(cfg.spec.clone())
        } else {
            bail!("Failed to find config for {}", self.region);
        }
    }
    pub fn get_versions(&self) -> Result<VersionMap> {
        let res = self.manifests.read()?
            .into_iter()
            .fold(BTreeMap::new(), |mut acc, crd| {
                acc.insert(crd.spec.name, crd.spec.version.unwrap());
                acc
            });
        Ok(res)
    }
    pub fn get_region(&self) -> Result<Region> {
        let cfg = self.get_config()?;
        match cfg.get_region(&self.region) {
            Ok(r) => Ok(r),
            Err(e) => bail!("could not resolve cluster for {}: {}", self.region, e)
        }
    }
    pub fn get_manifest(&self, key: &str) -> Result<Option<ManifestObject>> {
        let opt = self.manifests.read()?
            .into_iter()
            .find(|o| o.spec.name == key);
        Ok(opt)
    }
    pub fn get_manifests_for(&self, team: &str) -> Result<Vec<String>> {
        let mfs = self.manifests.read()?.into_iter()
            .filter(|crd| crd.spec.metadata.clone().unwrap().team == team)
            .map(|crd| crd.spec.name.clone()).collect();
        Ok(mfs)
    }
    pub fn get_reverse_deps(&self, service: &str) -> Result<Vec<String>> {
        let mut res = vec![];
        for crd in &self.manifests.read()? {
            if crd.spec.dependencies.iter().any(|d| d.name == service) {
                res.push(crd.spec.name.clone())
            }
        }
        Ok(res)
    }
    pub fn get_newrelic_link(&self, service: &str) -> Option<String> {
        self.relics.get(service).map(String::to_owned)
    }
    pub fn get_sentry_slug(&self, service: &str) -> Option<String> {
        self.sentries.get(service).map(String::to_owned)
    }

    // Interface for internal thread
    fn poll(&self) -> Result<()> {
        self.manifests.poll()?;
        self.configs.poll()?;
        Ok(())
    }

    fn update_slow_cache(&mut self) -> Result<()> {
        let region = self.get_region()?;
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

/// Initiailize state machine for an actix app
///
/// Returns a Sync
pub fn init(cfg: Configuration) -> Result<State> {
    let client = APIClient::new(cfg);
    let state = State::new(client)?; // for app to read
    let state_clone = state.clone(); // clone for internal thread
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(30));
            // update state here - can cause a few more waits in edge cases
            state_clone.poll().map_err(|e| {
                // Can't recover: boot as much as kubernetes' backoff allows
                error!("Failed to refesh cache '{}' - rebooting", e);
                std::process::exit(1); // boot might fix it if network is failing
            }).unwrap();
        }
    });
    Ok(state)
}
