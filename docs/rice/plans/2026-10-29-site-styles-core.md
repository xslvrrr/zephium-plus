# Page Transparency and Site Styles Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let web pages be transparent over the window material and texture, and apply Zen Internet's per-site styles (bundled snapshot plus daily updates) natively in WebKit.

**Architecture:** A pure crate (`zephium-site-styles`) parses `styles.json`, matches a host to its sheet and composes the CSS for the current settings. The engine gains a process-wide resolver (host → CSS) and installs one private `_WKUserStyleSheet` per host on a page view's user content controller when a main-frame navigation starts there. Page views stop painting WebKit's default background; a user-level backstop keeps unstyled pages opaque. The desktop owns the catalog, the bundled snapshot, the cache and the updater.

**Tech Stack:** Rust (serde, flate2 1.1.9, reqwest 0.13.4, objc2 / WebKit private API), Tauri 2.11 resources.

**Spec:** `docs/rice/2026-10-29-notch-and-site-styles-design.md` (§2, §3.1-§3.3, §3.5, §4).

## Global Constraints

- Do not run test suites. Verification is limited to relevant typechecks and builds: `cargo check` and `cargo clippy --all-targets` for touched crates; `cargo xtask check-agentic-probe-boundary`, `check-page-reader-boundary`, `check-page-assist-boundary`, `check-release-engine-security` when engine WebKit code changes. Unit tests may be written; they are not run. **Ruling (spike):** Task 1's spike runs one example binary once to observe WebKit behaviour; that is a probe, not a test suite.
- **Ruling (bindings):** regenerate `frame/src/shared/ipc/bindings.ts` only with `cargo test -p zephium-desktop --lib export_typescript_bindings`; never hand-edit it. (This plan adds no Tauri commands; plan 3 does.)
- No new crates.io dependencies: `serde`, `serde_json`, `thiserror`, `flate2 =1.1.9`, `reqwest =0.13.4` are already in the workspace or the lockfile.
- Source data: `sameerasw/my-internet` `styles.json`, MIT, "Copyright (c) 2025 Transparent Zen". Update URL exactly `https://sameerasw.github.io/my-internet/styles.json`. No other host is contacted.
- Fetched CSS never loads anything: features containing `@import`, `@font-face`, `image-set(`, or `url(` not followed by `data:` or `#` are dropped.
- Response cap 4 MiB; update interval 24 h; requests carry no cookies.
- The built `.app` must stay under 50 MiB.
- New native code is macOS-only behind `#[cfg(target_os = "macos")]`; other platforms compile.
- Every upstream file edited gets one row in `docs/rice/UPSTREAM-TOUCHPOINTS.md`.
- Commit after each task, message starting `rice:`, trailers:
  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01ML2gzDQx5BeuAPZFT135zZ
  ```
- Never push. Never touch `~/Library/Application Support/app.zephium` except `rice-theme.json`.

---

### Task 1: Spike `_WKUserStyleSheet`

**Files:**
- Create: `crates/zephium-engine/examples/rice_user_style_sheet.rs` (deleted again in Step 4)
- Create: `docs/rice/spikes/2026-10-29-user-style-sheet.md`

**Interfaces:**
- Produces: a recorded answer, in the spike note, to: does `_WKUserStyleSheet` exist on this macOS; which initialiser works; does an author-level sheet with include pattern `*://example.com/*` added during `webView:decidePolicyForNavigationAction:decisionHandler:` apply to that navigation's document; does `_removeUserStyleSheet:` remove it; does a user-level sheet lose to page CSS. Task 4 builds on the answer.

- [ ] **Step 1:** Read the selectors available at runtime: in the example, `objc2::runtime::AnyClass::get(c"_WKUserStyleSheet")` must be `Some`; print whether it responds to `initWithSource:forWKWebView:forMainFrameOnly:includeMatchPatternStrings:excludeMatchPatternStrings:baseURL:level:contentWorld:` and to `initWithSource:forMainFrameOnly:` (`class.responds_to(sel!(...))` via `instancesRespondToSelector:`), and whether `WKUserContentController` responds to `_addUserStyleSheet:`, `_removeUserStyleSheet:`, `_removeAllUserStyleSheets`. `_WKUserStyleLevel`: `0` = user, `1` = author.

- [ ] **Step 2:** In the same example, open a tao window with a wry webview (follow `crates/zephium-engine/src/platform/macos/agentic_input_probe.rs:766` for a minimal builder) loading `data:text/html,<body style="background:red">` is not matchable, so instead serve a local page: use wry's custom protocol `rice://example.com/` returning `<html><body style="background: rgb(255,0,0)">x</body></html>`, include pattern `rice://example.com/*`. Add an author-level sheet `body { background: rgb(0,0,255) !important; }` before loading and read back `getComputedStyle(document.body).backgroundColor` with `evaluate_script_with_callback`; then repeat adding the sheet from the navigation delegate's decide-policy point if reachable (if wry's delegate is not reachable from an example, record that the sheet was added before `load_url` and that Task 4 must find the engine's policy hook). Then add a user-level `:root { background-color: Canvas; }` sheet and confirm it does not override `background: rgb(255,0,0)`.

- [ ] **Step 3:** Run it once: `cargo run -p zephium-engine --example rice_user_style_sheet`. Write `docs/rice/spikes/2026-10-29-user-style-sheet.md` with the printed results and a verdict: **use `_WKUserStyleSheet`** (with the working initialiser and the hook point) or **fall back** (spec §3.3: an isolated-world `WKUserScript` per navigation that adopts the host's CSS as a constructed stylesheet).

- [ ] **Step 4:** Delete the example file (it must not ship). Commit the spike note only: `rice: spike _WKUserStyleSheet`.

---

### Task 2: Crate `zephium-site-styles`

**Files:**
- Create: `crates/zephium-site-styles/Cargo.toml`
- Create: `crates/zephium-site-styles/src/lib.rs`
- Modify: `Cargo.toml` (`[workspace.dependencies]` entry; the `crates/*` glob already makes it a member)
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md` (update the `Cargo.toml` row)

**Interfaces:**
- Produces: `Catalog::parse(&[u8]) -> Result<Catalog, Error>`, `Catalog::site_for(&self, host: &str) -> Option<&Site>`, `Catalog::css_for(&self, host: &str, settings: Settings, overrides: &Overrides) -> Option<String>`, `Catalog::len()`, `Catalog::dropped()`, `normalize_host(&str) -> Option<String>`, types `Site { key, features }`, `Feature { key, title, description, css, transparency }`, `Settings { enabled, force, transparency }`, `Overrides { sites: BTreeMap<String, SiteOverride> }`, `SiteOverride { enabled: Option<bool>, force: Option<bool>, features: BTreeMap<String, bool> }`, `FORCE_CSS`.

- [ ] **Step 1: Manifest.**

```toml
[package]
name = "zephium-site-styles"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
```

Add `zephium-site-styles = { path = "crates/zephium-site-styles", version = "=0.1.0" }` to `[workspace.dependencies]` beside `zephium-mcp-server`.

- [ ] **Step 2: Library.** `src/lib.rs`:

```rust
//! rice fork: Zen Internet's per-site styles (sameerasw/my-internet, MIT):
//! reading its `styles.json`, finding the sheet a host uses, and composing
//! the CSS to apply there for the person's settings.

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{Deserializer, MapAccess, Visitor};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("styles.json is not valid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("styles.json has {0} websites, outside 1..=5000")]
    Size(usize),
}

/// The forced transparency for a site with no style of its own. Written for
/// this fork.
pub const FORCE_CSS: &str = "html, body, body > div:only-child, #root, #app, main {\n  background: transparent !important;\n  background-color: transparent !important;\n}\n";

#[derive(Clone, Debug, PartialEq)]
pub struct Feature {
    /// The key in `styles.json`, which overrides are stored under.
    pub key: String,
    pub title: String,
    pub description: Option<String>,
    pub css: String,
    /// The site's transparency feature, which page transparency governs.
    pub transparency: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Site {
    /// The file name in `styles.json`, `+adobe.com.css`, which overrides are
    /// stored under.
    pub key: String,
    pub features: Vec<Feature>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub enabled: bool,
    pub force: bool,
    pub transparency: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub features: BTreeMap<String, bool>,
}

/// Only changes from the defaults, keyed by site key, or by host for a site
/// with no style.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Overrides {
    #[serde(default)]
    pub sites: BTreeMap<String, SiteOverride>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Rule {
    Exact,
    Subdomains,
    Suffixes,
}

#[derive(Clone, Debug)]
struct Entry {
    domain: String,
    rule: Rule,
    site: String,
    mapped: bool,
}

pub struct Catalog {
    sites: BTreeMap<String, Site>,
    entries: Vec<Entry>,
    dropped: usize,
}

/// A map whose values are strings, read in document order: a site's first
/// feature is its transparency feature by convention.
struct Ordered(Vec<(String, String)>);

impl<'de> Deserialize<'de> for Ordered {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Entries;
        impl<'de> Visitor<'de> for Entries {
            type Value = Ordered;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a map of strings")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Ordered, A::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry::<String, String>()? {
                    entries.push(entry);
                }
                Ok(Ordered(entries))
            }
        }
        deserializer.deserialize_map(Entries)
    }
}

#[derive(Deserialize)]
struct Raw {
    website: BTreeMap<String, Ordered>,
    #[serde(default)]
    mapping: BTreeMap<String, Vec<String>>,
}

/// Lower-case, without a trailing dot or a leading `www.`; `None` for
/// something that is not a host name.
pub fn normalize_host(host: &str) -> Option<String> {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_owned();
    let valid = !host.is_empty()
        && host.len() <= 253
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.');
    valid.then_some(host)
}

fn pattern(name: &str) -> Option<(Rule, String)> {
    let name = name.strip_suffix(".css").unwrap_or(name);
    let (rule, domain) = match name.as_bytes().first() {
        Some(b'+') => (Rule::Subdomains, &name[1..]),
        Some(b'-') => (Rule::Suffixes, &name[1..]),
        _ => (Rule::Exact, name),
    };
    normalize_host(domain).map(|domain| (rule, domain))
}

/// Whether a stylesheet would make a request: a third party's CSS on every
/// matching page must not be able to send the page anywhere.
fn loads(css: &str) -> bool {
    let css = css.to_ascii_lowercase();
    if css.contains("@import") || css.contains("@font-face") || css.contains("image-set(") {
        return true;
    }
    css.match_indices("url(").any(|(at, _)| {
        let rest = css[at + 4..].trim_start().trim_start_matches(['"', '\'']);
        !(rest.starts_with("data:") || rest.starts_with('#'))
    })
}

fn feature(key: String, css: String) -> Feature {
    let (head, description) = match key.split_once('$') {
        Some((head, description)) => (head.trim(), Some(description.trim().to_owned())),
        None => (key.trim(), None),
    };
    // A short lower-case prefix names the site's features (`yt-`).
    let title = match head.split_once('-') {
        Some((prefix, rest))
            if !rest.is_empty()
                && prefix.len() <= 8
                && prefix.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()) =>
        {
            rest.trim()
        }
        _ => head,
    }
    .to_owned();
    let transparency = title.eq_ignore_ascii_case("transparency");
    Feature {
        key,
        title,
        description: description.filter(|d| !d.is_empty()),
        css,
        transparency,
    }
}

fn matches(entry: &Entry, host: &str) -> bool {
    match entry.rule {
        Rule::Exact => host == entry.domain,
        Rule::Subdomains => {
            host == entry.domain
                || host
                    .strip_suffix(&entry.domain)
                    .is_some_and(|head| head.ends_with('.'))
        }
        // `-cifraclub.com.br` also covers `cifraclub.com`: the same first
        // label under any public suffix.
        Rule::Suffixes => {
            let base = entry.domain.split('.').next().unwrap_or_default();
            host.split_once('.').is_some_and(|(first, _)| first == base)
        }
    }
}

impl Catalog {
    pub fn parse(json: &[u8]) -> Result<Self, Error> {
        let raw: Raw = serde_json::from_slice(json)?;
        if raw.website.is_empty() || raw.website.len() > 5000 {
            return Err(Error::Size(raw.website.len()));
        }
        let mut dropped = 0;
        let mut sites = BTreeMap::new();
        let mut entries = Vec::new();
        for (key, Ordered(features)) in raw.website {
            let Some((rule, domain)) = pattern(&key) else {
                continue;
            };
            let features: Vec<Feature> = features
                .into_iter()
                .filter(|(_, css)| {
                    let loads = loads(css);
                    dropped += usize::from(loads);
                    !loads
                })
                .map(|(key, css)| feature(key, css))
                .collect();
            if features.is_empty() {
                continue;
            }
            entries.push(Entry { domain, rule, site: key.clone(), mapped: false });
            sites.insert(key.clone(), Site { key, features });
        }
        for (site, domains) in raw.mapping {
            if !sites.contains_key(&site) {
                continue;
            }
            for name in domains {
                if let Some((rule, domain)) = pattern(&name) {
                    entries.push(Entry { domain, rule, site: site.clone(), mapped: true });
                }
            }
        }
        Ok(Self { sites, entries, dropped })
    }

    pub fn len(&self) -> usize {
        self.sites.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sites.is_empty()
    }

    /// Features dropped because their CSS would make a request.
    pub fn dropped(&self) -> usize {
        self.dropped
    }

    /// The most specific match: a site's own name before a mapping, then
    /// exact before subdomains before other suffixes, then the longest name.
    pub fn site_for(&self, host: &str) -> Option<&Site> {
        let host = normalize_host(host)?;
        self.entries
            .iter()
            .filter(|entry| matches(entry, &host))
            .min_by_key(|entry| (entry.mapped, entry.rule, std::cmp::Reverse(entry.domain.len())))
            .and_then(|entry| self.sites.get(&entry.site))
    }

    /// The CSS to apply on `host`, or `None` for nothing.
    pub fn css_for(&self, host: &str, settings: Settings, overrides: &Overrides) -> Option<String> {
        if !settings.enabled {
            return None;
        }
        let Some(site) = self.site_for(host) else {
            let host = normalize_host(host)?;
            let chosen = overrides.sites.get(&host);
            if chosen.and_then(|o| o.enabled) == Some(false) {
                return None;
            }
            let force = chosen.and_then(|o| o.force).unwrap_or(settings.force);
            return (force && settings.transparency).then(|| FORCE_CSS.to_owned());
        };
        let chosen = overrides.sites.get(&site.key);
        if chosen.and_then(|o| o.enabled) == Some(false) {
            return None;
        }
        let css: Vec<&str> = site
            .features
            .iter()
            .filter(|feature| {
                chosen
                    .and_then(|o| o.features.get(&feature.key).copied())
                    .unwrap_or(!feature.transparency || settings.transparency)
            })
            .map(|feature| feature.css.as_str())
            .collect();
        (!css.is_empty()).then(|| css.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = r##"{
      "website": {
        "youtube.com.css": {
          "yt-Transparency": "html { background: none !important; }",
          "yt-Hide chips $ Hide the chips": "#chips { display: none; }",
          "yt-Leak": "body { background: url(https://evil.test/x); }"
        },
        "+adobe.com.css": { "transparency": "body { color: red; }" },
        "-cifraclub.com.br.css": { "transparency": "a {}" },
        "example.com.css": { "transparency": "p {}" }
      },
      "mapping": { "example.com.css": ["alternativeto.net"] }
    }"##;

    fn catalog() -> Catalog {
        Catalog::parse(JSON.as_bytes()).unwrap()
    }

    const ON: Settings = Settings { enabled: true, force: false, transparency: true };

    #[test]
    fn matches_by_rule() {
        let c = catalog();
        assert_eq!(c.site_for("www.youtube.com").unwrap().key, "youtube.com.css");
        assert!(c.site_for("m.youtube.com").is_none());
        assert_eq!(c.site_for("helpx.adobe.com").unwrap().key, "+adobe.com.css");
        assert_eq!(c.site_for("cifraclub.com").unwrap().key, "-cifraclub.com.br.css");
        assert_eq!(c.site_for("alternativeto.net").unwrap().key, "example.com.css");
    }

    #[test]
    fn parses_titles_and_drops_loads() {
        let c = catalog();
        let site = c.site_for("youtube.com").unwrap();
        assert_eq!(site.features.len(), 2);
        assert!(site.features[0].transparency);
        assert_eq!(site.features[1].title, "Hide chips");
        assert_eq!(site.features[1].description.as_deref(), Some("Hide the chips"));
        assert_eq!(c.dropped(), 1);
    }

    #[test]
    fn composes_for_settings_and_overrides() {
        let c = catalog();
        let none = Overrides::default();
        assert!(c.css_for("youtube.com", ON, &none).unwrap().contains("#chips"));
        let opaque = Settings { transparency: false, ..ON };
        assert!(!c.css_for("youtube.com", opaque, &none).unwrap().contains("html"));
        let mut off = Overrides::default();
        off.sites.insert("youtube.com.css".into(), SiteOverride { enabled: Some(false), ..Default::default() });
        assert!(c.css_for("youtube.com", ON, &off).is_none());
        assert!(c.css_for("unknown.test", ON, &none).is_none());
        let forced = Settings { force: true, ..ON };
        assert_eq!(c.css_for("unknown.test", forced, &none).as_deref(), Some(FORCE_CSS));
    }
}
```

- [ ] **Step 3: Verify.** `cargo check -p zephium-site-styles`, `cargo clippy -p zephium-site-styles --all-targets`, `cargo check -p zephium-site-styles --target x86_64-pc-windows-msvc`.

- [ ] **Step 4: Commit** `rice: zephium-site-styles crate`.

---

### Task 3: Bundled snapshot, credit and the desktop catalog

**Files:**
- Create: `desktop/resources/site-styles/styles.json.gz`
- Create: `desktop/resources/site-styles/LICENSE-my-internet.txt`
- Create: `scripts/rice-update-site-styles.sh`
- Modify: `desktop/tauri.macos.conf.json` (`bundle.resources`)
- Modify: `desktop/Cargo.toml` (`zephium-site-styles.workspace = true`, `flate2 = "=1.1.9"` if not already a desktop dependency)
- Create: `desktop/src/site_styles.rs`
- Modify: `desktop/src/lib.rs` (`mod site_styles;`, `site_styles::start(data_dir, app)` beside `rice_theme::start`)
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md`

**Interfaces:**
- Consumes: `zephium_site_styles::{Catalog, Overrides, Settings}`.
- Produces: `site_styles::catalog() -> Option<Arc<Catalog>>`, `site_styles::overrides() -> Overrides`, `site_styles::settings() -> Settings`, `site_styles::replace_catalog(Arc<Catalog>)`, `site_styles::save_overrides(Overrides) -> std::io::Result<()>`, `site_styles::status() -> Status { sites: usize, updated: Option<SystemTime> }`, constant `CACHE_DIR = "site-styles"` under the app data dir.

- [ ] **Step 1: Snapshot script.** `scripts/rice-update-site-styles.sh`:

```sh
#!/bin/sh
# rice fork: refreshes the bundled snapshot of Zen Internet's site styles
# (sameerasw/my-internet, MIT). Run by hand; the app also updates itself.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
out="$root/desktop/resources/site-styles"
mkdir -p "$out"
curl -sfL --max-filesize 4194304 https://sameerasw.github.io/my-internet/styles.json -o "$out/styles.json.tmp"
python3 -I -c "import json,sys; d=json.load(open(sys.argv[1])); assert 1 <= len(d['website']) <= 5000" "$out/styles.json.tmp"
gzip -9 -n -c "$out/styles.json.tmp" > "$out/styles.json.gz"
rm "$out/styles.json.tmp"
curl -sfL https://raw.githubusercontent.com/sameerasw/my-internet/main/LICENSE -o "$out/LICENSE-my-internet.txt"
```

Run it once and commit its two outputs.

- [ ] **Step 2: Resources.** In `desktop/tauri.macos.conf.json` add under `bundle` (JSON merge patch merges into the base `resources` object):

```json
"resources": {
  "resources/site-styles/styles.json.gz": "site-styles/styles.json.gz",
  "resources/site-styles/LICENSE-my-internet.txt": "licenses/site-styles/my-internet-MIT.txt"
}
```

- [ ] **Step 3: Desktop catalog.** `desktop/src/site_styles.rs`:
  - Module note: owns the site-styles catalog (cache over snapshot), the person's overrides, and (Task 5) the updater.
  - `static STATE: OnceLock<RwLock<State>>` with `catalog: Option<Arc<Catalog>>`, `overrides: Overrides`, `dir: PathBuf`, `updated: Option<SystemTime>`.
  - `start(data_dir: PathBuf, app: AppHandle)`: on a background thread, load `data_dir/site-styles/styles.json` (cache; also read `styles.json.etag` and its mtime as `updated`) if present and `Catalog::parse` succeeds; otherwise decompress the bundled resource (`app.path().resolve("site-styles/styles.json.gz", BaseDirectory::Resource)`, `flate2::read::GzDecoder`, read at most 8 MiB) and parse it. Load `data_dir/site-styles/overrides.json` (missing or invalid: default, logged with `crate::write_diagnostic`). Log `site-styles: N sites, M features dropped`. Then call `crate::site_styles::install_resolver()` (Task 4) and, on macOS, `start_updates` (Task 5).
  - `settings()`: `enabled = app_setting("site-styles.enabled") == "true"`, `force = app_setting("site-styles.force") == "true"`, `transparency = app_setting("ui.page-transparency") == "true"`. Until plan 3 adds these keys, `crate::app_setting` returns the stored value or `""`, so all are false and nothing applies.
  - `save_overrides`: write `overrides.json` atomically (temp file + rename) with `serde_json::to_vec_pretty`.

- [ ] **Step 4:** Touchpoints rows for `desktop/tauri.macos.conf.json`, `desktop/Cargo.toml`, `desktop/src/lib.rs`, `Cargo.lock` (new path dependency only).

- [ ] **Step 5: Verify.** `cargo check -p zephium-desktop`, clippy, `du -h desktop/resources/site-styles/styles.json.gz` (about 130 KB).

- [ ] **Step 6: Commit** `rice: bundle Zen Internet's site styles and load them`.

---

### Task 4: Page transparency and per-host style sheets in the engine

**Files:**
- Modify: `crates/zephium-engine/src/lib.rs` (`PAGE_TRANSPARENT` static with `set_page_transparent` / `page_transparent`; `SiteStyleResolver` registration; `refresh_site_styles`)
- Create: `crates/zephium-engine/src/platform/macos/site_sheets.rs`
- Modify: `crates/zephium-engine/src/platform/macos/native.rs` (`configure` applies transparency and attaches the backstop)
- Modify: the engine's main-frame navigation policy hook for page views (found in Step 1)
- Modify: `desktop/src/site_styles.rs` (`install_resolver`)
- Modify: `desktop/src/lib.rs` (seed `set_page_transparent` beside `set_page_radius`; projection arms for `ui.page-transparency`, `site-styles.enabled`, `site-styles.force` call `zephium_engine::refresh_site_styles()`)
- Modify: `xtask/src/*boundary*.rs` if a boundary check forbids the new WebKit calls (add exact allow-list entries with a `rice fork:` comment)
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md`

**Interfaces:**
- Consumes: the spike verdict in `docs/rice/spikes/2026-10-29-user-style-sheet.md`; `Catalog::css_for`.
- Produces: `zephium_engine::set_page_transparent(bool)`, `zephium_engine::page_transparent() -> bool`, `zephium_engine::set_site_style_resolver(Box<dyn Fn(&str) -> Option<String> + Send + Sync>)`, `zephium_engine::refresh_site_styles()`.

- [ ] **Step 1: Find the hook.** `rg -n "decidePolicyForNavigationAction|didReceiveServerRedirectForProvisionalNavigation|didStartProvisionalNavigation" crates/zephium-engine/src vendor/wry/src/wkwebview`. The style sheet must be added for a page view's main-frame navigation before its document is created: prefer the engine-side point that already sees each Browse main-frame navigation (decide-policy for navigation actions, plus server redirects). Record in the task report which hook you used and why.

- [ ] **Step 2: Statics.** In `crates/zephium-engine/src/lib.rs`, beside `PAGE_RADIUS`:

```rust
static PAGE_TRANSPARENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// rice fork: whether new Browse pages leave unpainted pixels clear, so the
/// window shows through where a page draws nothing.
pub fn set_page_transparent(transparent: bool) {
    PAGE_TRANSPARENT.store(transparent, std::sync::atomic::Ordering::Relaxed);
}

pub fn page_transparent() -> bool {
    PAGE_TRANSPARENT.load(std::sync::atomic::Ordering::Relaxed)
}

type SiteStyleResolver = Box<dyn Fn(&str) -> Option<String> + Send + Sync>;
static SITE_STYLES: std::sync::RwLock<Option<SiteStyleResolver>> = std::sync::RwLock::new(None);

/// rice fork: the CSS Browse pages on a host are given (Zen Internet's site
/// styles), decided by the desktop.
pub fn set_site_style_resolver(resolver: SiteStyleResolver) {
    if let Ok(mut slot) = SITE_STYLES.write() {
        *slot = Some(resolver);
    }
}

pub(crate) fn site_style_for(host: &str) -> Option<String> {
    SITE_STYLES.read().ok()?.as_ref()?(host)
}
```

`refresh_site_styles()` schedules, on the engine's main thread (the same `run` path other host updates use), `platform::macos::site_sheets::refresh_all()`; elsewhere a no-op.

- [ ] **Step 3: Sheets (verdict: `_WKUserStyleSheet`).** `site_sheets.rs`:
  - Module note: per-host author sheets for site styles and one user-level backstop; private WebKit API, macOS only; why per host (spec §3.3).
  - A thread-local registry: for each live page `WKWebView` (weak), the host its fork sheet is for and the retained sheet object; plus its backstop sheet.
  - `attach(wk: &WKWebView)` (called from `configure`): if `page_transparent()`, set `drawsBackground` to `NO` with `setValue_forKey(Some(&NSNumber::new_bool(false)), ns_string!("drawsBackground"))` on the view, `setUnderPageBackgroundColor(Some(&NSColor::clearColor()))`, and add the backstop: a user-level (`level = 0`), all-frames sheet `:root { background-color: Canvas; }` with no include patterns. Register the view.
  - `navigating(wk: &WKWebView, url: &NSURL)` (called from the Step 1 hook, main frame only): take `url.host()`; if it equals the registered host, return. Otherwise remove the view's fork sheet (`_removeUserStyleSheet:`), and if `site_style_for(host)` is `Some(css)`, create an author-level (`level = 1`), main-frame-only sheet with include pattern `*://<host>/*` and add it (`_addUserStyleSheet:`); register host and sheet.
  - `refresh_all()`: for every registered view, drop its fork sheet and run `navigating` with its current `URL()`; re-check `page_transparent()` and add or remove the backstop and background accordingly (pages already loaded update on reload).
  - All `msg_send!` calls check `respondsToSelector:` first; when the class or selector is missing, log once (`eprintln!("engine: _WKUserStyleSheet unavailable")`) and do nothing.
  - If the spike verdict was **fall back**, implement `navigating` instead by evaluating, in a dedicated `WKContentWorld` named `rice-site-styles`, a script that sets `document.adoptedStyleSheets` to include one `CSSStyleSheet` built from the CSS (passed as a JSON string literal), on `didCommitNavigation` for the main frame, and record in the touchpoints row that pages may flash before it applies.

- [ ] **Step 4: Configure.** In `native.rs` `configure`, after the existing setup and before returning `Ok(())`, call `super::site_sheets::attach(&wk)`, behind `#[cfg(target_os = "macos")]` (the file is macOS-only already).

- [ ] **Step 5: Material guard.** In `desktop/src/lib.rs`, seed `zephium_engine::set_page_transparent(app_setting("ui.page-transparency") == "true" && app_setting("ui.material") != "none")` next to the `set_page_radius` seeding, and update it in the projection hook for `preference.ui.page-transparency=` and `preference.ui.material=` before calling `zephium_engine::refresh_site_styles()`.

- [ ] **Step 6: Resolver.** `site_styles::install_resolver()`:

```rust
pub(crate) fn install_resolver() {
    zephium_engine::set_site_style_resolver(Box::new(|host| {
        let state = STATE.get()?.read().ok()?;
        state.catalog.as_ref()?.css_for(host, settings(), &state.overrides)
    }));
    zephium_engine::refresh_site_styles();
}
```

Call `refresh_site_styles()` after `replace_catalog` and `save_overrides` too.

- [ ] **Step 7: Boundaries.** Run `cargo xtask check-agentic-probe-boundary`, `check-page-reader-boundary`, `check-page-assist-boundary`, `check-release-engine-security`. If one rejects the new calls, add the narrowest allow-list entry with a `rice fork:` comment.

- [ ] **Step 8:** Touchpoints rows for `zephium-engine/src/lib.rs`, `native.rs`, the hook file, `desktop/src/lib.rs`, any `xtask` file.

- [ ] **Step 9: Verify.** `cargo check -p zephium-engine -p zephium-desktop`, clippy the same, `cargo check -p zephium-engine --target x86_64-pc-windows-msvc`, the four `xtask` checks.

- [ ] **Step 10: Commit** `rice: transparent pages and per-host site style sheets`.

---

### Task 5: Daily updates

**Files:**
- Modify: `desktop/src/site_styles.rs`
- Modify: `docs/rice/UPSTREAM-TOUCHPOINTS.md` (only if an upstream file changes)

**Interfaces:**
- Produces: `site_styles::update_now() -> impl Future<Output = Result<UpdateOutcome, String>>` with `UpdateOutcome::{Updated(usize), Unchanged, Skipped}`; the 24 h loop; `status()` reflects the last success.

- [ ] **Step 1: Fetch.**

```rust
const URL: &str = "https://sameerasw.github.io/my-internet/styles.json";
const MAX_BYTES: usize = 4 * 1024 * 1024;
const EVERY: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);
```

Build one `reqwest::Client` with `.https_only(true)`, `.redirect(reqwest::redirect::Policy::limited(3))`, `.timeout(Duration::from_secs(30))`, user agent `Zephium-rice`, no cookie store (reqwest's default). Request with `If-None-Match` from `styles.json.etag` when present. `304`: `Unchanged`, touch the cache's mtime so `updated` advances. `200`: stream the body with a running length check, aborting past `MAX_BYTES`; `Catalog::parse`; on success write `styles.json` and `styles.json.etag` atomically, `replace_catalog`, log `site-styles: updated, N sites`. Any other status or error: keep the current catalog, log it, return `Err`.

- [ ] **Step 2: Loop.** `start_updates(app)`: `tauri::async_runtime::spawn` a loop that, while `app_setting("site-styles.auto-update") != "false"` and `app_setting("site-styles.enabled") == "true"`, runs `update_now` when the cache is older than `EVERY` (or missing), then sleeps until the next due time, re-checking at most hourly. `update_now` runs regardless of `auto-update` (it is the explicit button in plan 3), but never concurrently: a second call while one is running returns `Skipped`.

- [ ] **Step 3: Verify.** `cargo check -p zephium-desktop`, clippy.

- [ ] **Step 4: Commit** `rice: refresh site styles daily`.
