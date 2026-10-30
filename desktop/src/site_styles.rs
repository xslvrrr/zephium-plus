//! rice fork: Zen Internet's site styles (sameerasw/my-internet, MIT).
//!
//! Owns the catalog (the profile's cached `styles.json` over the bundled
//! snapshot), the person's per-site and per-feature overrides, and the daily
//! update. The engine asks the resolver installed here for the style of each
//! host a page navigates to.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{Duration, SystemTime};

use tauri::{AppHandle, Manager};
use zephium_site_styles::{feature_on, normalize_host, Catalog, Overrides, Settings, SiteOverride};

pub(crate) const CACHE_DIR: &str = "site-styles";
const CACHE: &str = "styles.json";
const ETAG: &str = "styles.json.etag";
const OVERRIDES: &str = "overrides.json";
const BUNDLED: &str = "site-styles/styles.json.gz";
const URL: &str = "https://sameerasw.github.io/my-internet/styles.json";
const UPDATE_HOST: &str = "sameerasw.github.io";
const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_BUNDLED_BYTES: u64 = 8 * 1024 * 1024;
const EVERY: Duration = Duration::from_secs(24 * 60 * 60);
const RECHECK: Duration = Duration::from_secs(60 * 60);

struct State {
    catalog: Option<Arc<Catalog>>,
    overrides: Overrides,
    dir: PathBuf,
    /// The last successful update or unchanged check; `None` while only the
    /// bundled snapshot is in use.
    updated: Option<SystemTime>,
    error: Option<String>,
}

static STATE: OnceLock<RwLock<State>> = OnceLock::new();
static UPDATING: AtomicBool = AtomicBool::new(false);

/// Preference changes arrive as projections, possibly ahead of the store's
/// write; each projected value shadows the stored one.
static PROJECTED: Mutex<BTreeMap<String, String>> = Mutex::new(BTreeMap::new());

/// The preferences that change what pages are given.
const KEYS: &[&str] = &[
    "ui.page-transparency",
    "ui.material",
    "site-styles.enabled",
    "site-styles.force",
];

fn setting(key: &str) -> Option<String> {
    PROJECTED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(key)
        .cloned()
        .or_else(|| crate::app_setting(key))
}

/// Records a projected preference. True when pages' styles depend on it, so
/// the caller refreshes them.
pub(crate) fn follow(key: &str, value: &str) -> bool {
    if !KEYS.contains(&key) && key != "site-styles.auto-update" {
        return false;
    }
    PROJECTED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key.to_owned(), value.to_owned());
    KEYS.contains(&key)
}

/// Page transparency needs a window material to show through; over the
/// Solid tier it is off.
pub(crate) fn settings() -> Settings {
    let on = |key| setting(key).as_deref() == Some("true");
    Settings {
        enabled: on("site-styles.enabled"),
        force: on("site-styles.force"),
        transparency: on("ui.page-transparency")
            && crate::material::preference() != crate::material::Preference::None,
    }
}

fn state() -> Option<&'static RwLock<State>> {
    STATE.get()
}

fn read_cache(dir: &Path) -> Option<(Catalog, Option<SystemTime>)> {
    let path = dir.join(CACHE);
    let bytes = std::fs::read(&path).ok()?;
    let updated = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    match Catalog::parse(&bytes) {
        Ok(catalog) => Some((catalog, updated)),
        Err(error) => {
            crate::write_diagnostic(format_args!("site-styles: cache ignored: {error}"));
            None
        }
    }
}

fn read_bundled(app: &AppHandle) -> Option<Catalog> {
    let path = app
        .path()
        .resolve(BUNDLED, tauri::path::BaseDirectory::Resource)
        .ok()?;
    let file = std::fs::File::open(&path).ok()?;
    let mut json = Vec::new();
    flate2::read::GzDecoder::new(file)
        .take(MAX_BUNDLED_BYTES)
        .read_to_end(&mut json)
        .ok()?;
    match Catalog::parse(&json) {
        Ok(catalog) => Some(catalog),
        Err(error) => {
            crate::write_diagnostic(format_args!(
                "site-styles: bundled snapshot invalid: {error}"
            ));
            None
        }
    }
}

fn read_overrides(dir: &Path) -> Overrides {
    match std::fs::read(dir.join(OVERRIDES)) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|error| {
            crate::write_diagnostic(format_args!("site-styles: overrides ignored: {error}"));
            Overrides::default()
        }),
        Err(_) => Overrides::default(),
    }
}

/// Writes `bytes` to `path` through a temporary file and a rename, so a
/// reader never sees half a file.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path)
}

pub(crate) fn start(data_dir: &Path, app: AppHandle) {
    let dir = data_dir.join(CACHE_DIR);
    std::thread::spawn(move || {
        let (catalog, updated) = match read_cache(&dir) {
            Some((catalog, updated)) => (Some(catalog), updated),
            None => (read_bundled(&app), None),
        };
        match &catalog {
            Some(catalog) => crate::write_diagnostic(format_args!(
                "site-styles: {} sites, {} features dropped",
                catalog.len(),
                catalog.dropped()
            )),
            None => crate::write_diagnostic(format_args!("site-styles: no catalog")),
        }
        let state = State {
            catalog: catalog.map(Arc::new),
            overrides: read_overrides(&dir),
            dir,
            updated,
            error: None,
        };
        if STATE.set(RwLock::new(state)).is_err() {
            return;
        }
        install_resolver();
        start_updates();
    });
}

fn install_resolver() {
    zephium_engine::set_site_style_resolver(Box::new(|host| {
        let state = state()?.read().ok()?;
        let style = state
            .catalog
            .as_ref()?
            .style_for(host, settings(), &state.overrides)?;
        Some((style.css, style.transparent))
    }));
    zephium_engine::refresh_site_styles();
}

fn replace_catalog(catalog: Catalog, updated: SystemTime) {
    if let Some(lock) = state() {
        if let Ok(mut state) = lock.write() {
            state.catalog = Some(Arc::new(catalog));
            state.updated = Some(updated);
            state.error = None;
        }
    }
    zephium_engine::refresh_site_styles();
}

fn save_overrides(overrides: Overrides) -> std::io::Result<()> {
    let lock = state().ok_or_else(|| std::io::Error::other("site styles are not loaded"))?;
    let dir = lock
        .read()
        .map_err(|_| std::io::Error::other("site styles are unavailable"))?
        .dir
        .clone();
    std::fs::create_dir_all(&dir)?;
    let bytes = serde_json::to_vec_pretty(&overrides).map_err(std::io::Error::other)?;
    write_atomic(&dir.join(OVERRIDES), &bytes)?;
    if let Ok(mut state) = lock.write() {
        state.overrides = overrides;
    }
    zephium_engine::refresh_site_styles();
    Ok(())
}

#[derive(Clone, Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FeatureView {
    pub key: String,
    pub title: String,
    pub description: Option<String>,
    pub on: bool,
    pub transparency: bool,
}

#[derive(Clone, Debug, Default, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SiteStylesView {
    /// The host, normalized; empty when the page has no host.
    pub host: String,
    /// Whether `styles.json` has a style for this host.
    pub styled: bool,
    pub enabled: bool,
    pub forced: bool,
    /// The person changed something here.
    pub changed: bool,
    pub features: Vec<FeatureView>,
}

#[derive(Clone, Debug, serde::Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SiteStylesChange {
    Enabled { on: bool },
    Forced { on: bool },
    Feature { key: String, on: bool },
    Reset,
}

#[derive(Clone, Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SiteStylesStatus {
    pub sites: u32,
    /// Milliseconds since the Unix epoch of the last successful update or
    /// unchanged check; `None` while only the bundled snapshot is in use.
    pub updated: Option<f64>,
    pub updating: bool,
    pub error: Option<String>,
}

/// The key overrides for `host` are stored under: its site's, or the host
/// itself for a site with no style.
fn override_key(catalog: Option<&Catalog>, host: &str) -> (String, bool) {
    match catalog.and_then(|catalog| catalog.site_for(host)) {
        Some(site) => (site.key.clone(), true),
        None => (host.to_owned(), false),
    }
}

pub(crate) fn view(host: &str) -> SiteStylesView {
    let Some(host) = normalize_host(host) else {
        return SiteStylesView::default();
    };
    let Some(state) = state().and_then(|lock| lock.read().ok()) else {
        return SiteStylesView {
            host,
            ..SiteStylesView::default()
        };
    };
    let settings = settings();
    let site = state
        .catalog
        .as_ref()
        .and_then(|catalog| catalog.site_for(&host));
    let (key, _) = override_key(state.catalog.as_deref(), &host);
    let chosen = state.overrides.sites.get(&key);
    let enabled = chosen.and_then(|o| o.enabled).unwrap_or(true);
    let changed = chosen.is_some_and(|o| !o.is_empty());
    match site {
        Some(site) => SiteStylesView {
            host,
            styled: true,
            enabled,
            forced: false,
            changed,
            features: site
                .features
                .iter()
                .map(|feature| FeatureView {
                    key: feature.key.clone(),
                    title: feature.title.clone(),
                    description: feature.description.clone(),
                    on: feature_on(feature, chosen, settings),
                    transparency: feature.transparency,
                })
                .collect(),
        },
        None => SiteStylesView {
            host,
            styled: false,
            enabled,
            forced: chosen.and_then(|o| o.force).unwrap_or(settings.force),
            changed,
            features: Vec::new(),
        },
    }
}

/// Applies one change for `host`, storing only values that differ from the
/// defaults, and returns the new view.
pub(crate) fn change(host: &str, change: SiteStylesChange) -> Result<SiteStylesView, String> {
    let Some(host) = normalize_host(host) else {
        return Ok(SiteStylesView::default());
    };
    let lock = state().ok_or("Site styles are still loading.")?;
    let mut overrides = {
        let state = lock.read().map_err(|_| "Site styles are unavailable.")?;
        state.overrides.clone()
    };
    let (key, styled) = {
        let state = lock.read().map_err(|_| "Site styles are unavailable.")?;
        override_key(state.catalog.as_deref(), &host)
    };
    let settings = settings();
    let entry = overrides.sites.entry(key.clone()).or_default();
    match change {
        SiteStylesChange::Enabled { on } => entry.enabled = (!on).then_some(false),
        SiteStylesChange::Forced { on } if !styled => {
            entry.force = (on != settings.force).then_some(on);
        }
        SiteStylesChange::Forced { .. } => {}
        SiteStylesChange::Feature { key: feature, on } if styled => {
            // Every feature defaults to on; transparency also needs page
            // transparency, which the switch cannot turn on here.
            if on {
                entry.features.remove(&feature);
            } else {
                entry.features.insert(feature, false);
            }
        }
        SiteStylesChange::Feature { .. } => {}
        SiteStylesChange::Reset => *entry = SiteOverride::default(),
    }
    if entry.is_empty() {
        overrides.sites.remove(&key);
    }
    save_overrides(overrides).map_err(|error| {
        crate::write_diagnostic(format_args!("site-styles: overrides not saved: {error}"));
        "The change could not be saved.".to_owned()
    })?;
    Ok(view(&host))
}

pub(crate) fn status_unavailable() -> SiteStylesStatus {
    SiteStylesStatus {
        sites: 0,
        updated: None,
        updating: false,
        error: None,
    }
}

pub(crate) fn status() -> SiteStylesStatus {
    let updating = UPDATING.load(Ordering::Acquire);
    let Some(state) = state().and_then(|lock| lock.read().ok()) else {
        return SiteStylesStatus {
            updating,
            ..status_unavailable()
        };
    };
    SiteStylesStatus {
        sites: state.catalog.as_ref().map_or(0, |catalog| {
            u32::try_from(catalog.len()).unwrap_or(u32::MAX)
        }),
        updated: state
            .updated
            .and_then(|at| at.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|since| since.as_millis() as f64),
        updating,
        error: state.error.clone(),
    }
}

#[derive(Debug)]
pub(crate) enum UpdateOutcome {
    Updated,
    Unchanged,
    Skipped,
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .https_only(true)
        // Only the styles' own host: no request leaves for anywhere else.
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() < 3 && attempt.url().host_str() == Some(UPDATE_HOST) {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .user_agent("Zephium-rice")
        .build()
        .map_err(|error| error.to_string())
}

fn record_error(error: &str) {
    crate::write_diagnostic(format_args!("site-styles: update failed: {error}"));
    if let Some(Ok(mut state)) = state().map(|lock| lock.write()) {
        state.error = Some(error.to_owned());
    }
}

/// Fetches `styles.json` once, keeping the current catalog on any failure.
/// A call while another runs returns `Skipped`.
pub(crate) async fn update_now() -> Result<UpdateOutcome, String> {
    if UPDATING.swap(true, Ordering::AcqRel) {
        return Ok(UpdateOutcome::Skipped);
    }
    let outcome = fetch().await;
    UPDATING.store(false, Ordering::Release);
    if let Err(error) = &outcome {
        record_error(error);
    }
    outcome
}

async fn fetch() -> Result<UpdateOutcome, String> {
    let dir = state()
        .and_then(|lock| lock.read().ok().map(|state| state.dir.clone()))
        .ok_or("Site styles are still loading.")?;
    let etag = std::fs::read_to_string(dir.join(ETAG))
        .ok()
        .filter(|_| dir.join(CACHE).is_file());
    let mut request = client()?.get(URL);
    if let Some(etag) = etag
        .as_deref()
        .map(str::trim)
        .filter(|etag| !etag.is_empty())
    {
        request = request.header(reqwest::header::IF_NONE_MATCH, etag);
    }
    let mut response = request.send().await.map_err(|error| error.to_string())?;
    let now = SystemTime::now();
    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        let cache = dir.join(CACHE);
        if let Ok(file) = std::fs::File::options().append(true).open(&cache) {
            let _ = file.set_modified(now);
        }
        if let Some(Ok(mut state)) = state().map(|lock| lock.write()) {
            state.updated = Some(now);
            state.error = None;
        }
        return Ok(UpdateOutcome::Unchanged);
    }
    if !response.status().is_success() {
        return Err(format!("the server answered {}", response.status()));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_BYTES as u64)
    {
        return Err("the styles are too large".to_owned());
    }
    let etag = response
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        if body.len() + chunk.len() > MAX_BYTES {
            return Err("the styles are too large".to_owned());
        }
        body.extend_from_slice(&chunk);
    }
    let catalog = Catalog::parse(&body).map_err(|error| error.to_string())?;
    let sites = catalog.len();
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    write_atomic(&dir.join(CACHE), &body).map_err(|error| error.to_string())?;
    match etag {
        Some(etag) => write_atomic(&dir.join(ETAG), etag.as_bytes()),
        None => std::fs::remove_file(dir.join(ETAG)).or(Ok(())),
    }
    .map_err(|error: std::io::Error| error.to_string())?;
    replace_catalog(catalog, now);
    crate::write_diagnostic(format_args!("site-styles: updated, {sites} sites"));
    Ok(UpdateOutcome::Updated)
}

fn auto_update() -> bool {
    setting("site-styles.auto-update").as_deref() != Some("false")
        && setting("site-styles.enabled").as_deref() == Some("true")
}

fn start_updates() {
    tauri::async_runtime::spawn(async {
        loop {
            let due = state()
                .and_then(|lock| lock.read().ok().map(|state| state.updated))
                .flatten()
                .and_then(|at| at.checked_add(EVERY))
                .is_none_or(|next| next <= SystemTime::now());
            if due && auto_update() {
                let _ = update_now().await;
            }
            tokio::time::sleep(RECHECK).await;
        }
    });
}
