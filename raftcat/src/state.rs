use failure::err_msg;
use kube::{
    api::{ListParams, Meta, Resource},
    client::APIClient,
    config::Configuration,
    runtime::Reflector,
};
use shipcat_definitions::{ShipcatConfig, ShipcatManifest};
use tera::compile_templates;

use std::{
    collections::BTreeMap,
    env,
    sync::{Arc, RwLock},
};

use crate::{
    integrations::{
        newrelic::{self, RelicMap},
        sentryapi::{self, SentryMap},
    },
    *,
};

/// Map of service -> versions
pub type VersionMap = BTreeMap<String, String>;

/// The canonical shared state for actix
///
/// Consumers of these (http handlers) should use public impls on this struct only.
/// Callers should not need to care about getting read/write locks.
/// Only this file should have a write handler to this struct.
#[derive(Clone)]
pub struct State {
    manifests: Reflector<ShipcatManifest>,
    configs: Reflector<ShipcatConfig>,
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
    pub async fn new(client: APIClient) -> Result<Self> {
        info!("Loading state from CRDs");
        let region = env::var("REGION_NAME").expect("Need REGION_NAME evar");
        let ns = env::var("NAMESPACE").expect("Need NAMESPACE evar");
        let t = compile_templates!(concat!("raftcat", "/templates/*"));
        debug!("Initializing cache for {} in {}", region, ns);

        let mfresource = Resource::namespaced::<ShipcatManifest>(&ns);
        let cfgresource = Resource::namespaced::<ShipcatConfig>(&ns);

        let lp = ListParams::default();
        let manifests = Reflector::new(client.clone(), lp.clone(), mfresource)
            .init()
            .await?;
        let configs = Reflector::new(client, lp, cfgresource).init().await?;
        // Use federated config if available:
        let is_federated = configs
            .state()
            .await?
            .iter()
            .any(|crd| Meta::name(crd) == "unionised");
        let config_name = if is_federated {
            "unionised".into()
        } else {
            region.clone()
        };
        let mut res = State {
            manifests,
            configs,
            region,
            config_name,
            relics: BTreeMap::new(),
            sentries: BTreeMap::new(),
            template: Arc::new(RwLock::new(t)),
        };
        res.update_slow_cache().await?;
        Ok(res)
    }

    /// Template getter for main
    pub fn render_template(&self, tpl: &str, ctx: tera::Context) -> String {
        let t = self.template.read().unwrap();
        t.render(tpl, &ctx).unwrap()
    }

    // Getters for main
    pub async fn get_manifests(&self) -> Result<BTreeMap<String, Manifest>> {
        let xs = self
            .manifests
            .state()
            .await?
            .into_iter()
            .fold(BTreeMap::new(), |mut acc, crd| {
                acc.insert(crd.spec.name.clone(), crd.spec); // don't expose crd metadata + status
                acc
            });
        Ok(xs)
    }

    pub async fn get_config(&self) -> Result<Config> {
        let cfgs = self.configs.state().await?;
        if let Some(cfg) = cfgs.into_iter().find(|c| Meta::name(c) == self.config_name) {
            Ok(cfg.spec)
        } else {
            bail!("Failed to find config for {}", self.region);
        }
    }

    pub async fn get_versions(&self) -> Result<VersionMap> {
        let res = self
            .manifests
            .state()
            .await?
            .into_iter()
            .fold(BTreeMap::new(), |mut acc, crd| {
                acc.insert(crd.spec.name, crd.spec.version.unwrap());
                acc
            });
        Ok(res)
    }

    pub async fn get_region(&self) -> Result<Region> {
        let cfg = self.get_config().await?;
        cfg.get_region(&self.region)
            .map_err(|e| err_msg(format!("could not resolve cluster for {}: {}", self.region, e)))
    }

    pub async fn get_manifest(&self, key: &str) -> Result<Option<ShipcatManifest>> {
        let opt = self
            .manifests
            .state()
            .await?
            .into_iter()
            .find(|o| o.spec.name == key);
        Ok(opt)
    }

    pub async fn get_manifests_for(&self, team: &str) -> Result<Vec<String>> {
        let mfs = self
            .manifests
            .state()
            .await?
            .into_iter()
            .filter(|crd| crd.spec.metadata.clone().unwrap().team == team)
            .map(|crd| crd.spec.name)
            .collect();
        Ok(mfs)
    }

    pub async fn get_reverse_deps(&self, service: &str) -> Result<Vec<String>> {
        let mut res = vec![];
        for crd in &self.manifests.state().await? {
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
    async fn poller(&self) -> Result<()> {
        let c = self.clone();
        // Make sure we always keep polling the reflectors.
        // If any of them fail, boot to let kubernete's backoff to hopefully fix it
        tokio::spawn(async move {
            loop {
                if let Err(e) = c.manifests.poll().await {
                    error!("Kube state failed to recover: {}", e);
                    std::process::exit(1);
                }
            }
        });
        let c2 = self.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = c2.configs.poll().await {
                    error!("Kube state failed to recover: {}", e);
                    std::process::exit(1);
                }
            }
        });
        Ok(())
    }

    async fn update_slow_cache(&mut self) -> Result<()> {
        let region = self.get_region().await?;
        if let Some(s) = region.sentry {
            match sentryapi::get_slugs(&s.url, &region.environment.to_string()).await {
                Ok(res) => {
                    self.sentries = res;
                    info!("Loaded {} sentry slugs", self.sentries.len());
                }
                Err(e) => warn!("Unable to load sentry slugs: {}", err_msg(e)),
            }
        } else {
            warn!("No sentry url configured for this region");
        }
        match newrelic::get_links(&region.name).await {
            Ok(res) => {
                self.relics = res;
                info!("Loaded {} newrelic links", self.relics.len());
            }
            Err(e) => warn!("Unable to load newrelic projects. {}", err_msg(e)),
        }
        Ok(())
    }
}

/// Initiailize state machine for an actix app
///
/// Returns a Sync
pub async fn init(cfg: Configuration) -> Result<State> {
    let client = APIClient::new(cfg);
    let state = State::new(client).await?;
    state.poller().await?; // starts inifinite polling tasks
    Ok(state)
}
