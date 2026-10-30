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
    /// A transparency feature, which page transparency governs.
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

impl SiteOverride {
    pub fn is_empty(&self) -> bool {
        self.enabled.is_none() && self.force.is_none() && self.features.is_empty()
    }
}

/// Only changes from the defaults, keyed by site key, or by host for a site
/// with no style.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Overrides {
    #[serde(default)]
    pub sites: BTreeMap<String, SiteOverride>,
}

/// What a host is given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Style {
    pub css: String,
    /// The style clears the page's background, so the page view should stop
    /// painting its own.
    pub transparent: bool,
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

/// A map whose values are strings, read in document order, so features keep
/// the order their site lists them in.
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
                && prefix
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()) =>
        {
            rest.trim()
        }
        _ => head,
    }
    .to_owned();
    // As in Zen Internet, any feature named for transparency is governed by
    // the transparency switch (`transparency`, `transparent-background`, and
    // one site's `tranparency`).
    let lower = title.to_ascii_lowercase();
    let transparency = lower.contains("transparen") || lower.contains("tranparen");
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
            entries.push(Entry {
                domain,
                rule,
                site: key.clone(),
                mapped: false,
            });
            sites.insert(key.clone(), Site { key, features });
        }
        for (site, domains) in raw.mapping {
            if !sites.contains_key(&site) {
                continue;
            }
            for name in domains {
                if let Some((rule, domain)) = pattern(&name) {
                    entries.push(Entry {
                        domain,
                        rule,
                        site: site.clone(),
                        mapped: true,
                    });
                }
            }
        }
        Ok(Self {
            sites,
            entries,
            dropped,
        })
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
            .min_by_key(|entry| {
                (
                    entry.mapped,
                    entry.rule,
                    std::cmp::Reverse(entry.domain.len()),
                )
            })
            .and_then(|entry| self.sites.get(&entry.site))
    }

    /// The style to apply on `host`, or `None` for nothing.
    pub fn style_for(
        &self,
        host: &str,
        settings: Settings,
        overrides: &Overrides,
    ) -> Option<Style> {
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
            return (force && settings.transparency).then(|| Style {
                css: FORCE_CSS.to_owned(),
                transparent: true,
            });
        };
        let chosen = overrides.sites.get(&site.key);
        if chosen.and_then(|o| o.enabled) == Some(false) {
            return None;
        }
        let on: Vec<&Feature> = site
            .features
            .iter()
            .filter(|feature| feature_on(feature, chosen, settings))
            .collect();
        if on.is_empty() {
            return None;
        }
        Some(Style {
            css: on
                .iter()
                .map(|feature| feature.css.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            transparent: on.iter().any(|feature| feature.transparency),
        })
    }
}

/// Whether `feature` applies: the person's choice, or Zen Internet's default
/// of every feature on, with transparency following page transparency.
pub fn feature_on(feature: &Feature, chosen: Option<&SiteOverride>, settings: Settings) -> bool {
    if feature.transparency && !settings.transparency {
        return false;
    }
    chosen
        .and_then(|o| o.features.get(&feature.key).copied())
        .unwrap_or(true)
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

    const ON: Settings = Settings {
        enabled: true,
        force: false,
        transparency: true,
    };

    #[test]
    fn matches_by_rule() {
        let c = catalog();
        assert_eq!(
            c.site_for("www.youtube.com").unwrap().key,
            "youtube.com.css"
        );
        assert!(c.site_for("m.youtube.com").is_none());
        assert_eq!(c.site_for("helpx.adobe.com").unwrap().key, "+adobe.com.css");
        assert_eq!(
            c.site_for("cifraclub.com").unwrap().key,
            "-cifraclub.com.br.css"
        );
        assert_eq!(
            c.site_for("alternativeto.net").unwrap().key,
            "example.com.css"
        );
    }

    #[test]
    fn parses_titles_and_drops_loads() {
        let c = catalog();
        let site = c.site_for("youtube.com").unwrap();
        assert_eq!(site.features.len(), 2);
        assert!(site.features[0].transparency);
        assert_eq!(site.features[1].title, "Hide chips");
        assert_eq!(
            site.features[1].description.as_deref(),
            Some("Hide the chips")
        );
        assert_eq!(c.dropped(), 1);
    }

    #[test]
    fn composes_for_settings_and_overrides() {
        let c = catalog();
        let none = Overrides::default();
        let style = c.style_for("youtube.com", ON, &none).unwrap();
        assert!(style.css.contains("#chips") && style.transparent);
        let opaque = Settings {
            transparency: false,
            ..ON
        };
        let style = c.style_for("youtube.com", opaque, &none).unwrap();
        assert!(!style.css.contains("html") && !style.transparent);
        let mut off = Overrides::default();
        off.sites.insert(
            "youtube.com.css".into(),
            SiteOverride {
                enabled: Some(false),
                ..Default::default()
            },
        );
        assert!(c.style_for("youtube.com", ON, &off).is_none());
        assert!(c.style_for("unknown.test", ON, &none).is_none());
        let forced = Settings { force: true, ..ON };
        assert_eq!(
            c.style_for("unknown.test", forced, &none),
            Some(Style {
                css: FORCE_CSS.to_owned(),
                transparent: true
            })
        );
    }
}
