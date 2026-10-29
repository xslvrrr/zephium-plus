//! rice fork: `rice-theme.json` beside the store, written by `rice theme
//! apply`. Each field present is applied as a preference at launch and on
//! every change of the file; a field left out leaves its preference alone.
//! A malformed file is logged and ignored whole; a field this build does not
//! know is logged and ignored alone, so a newer rice never voids the file.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::Deserialize;
use zephium_app::{Command, Handle};

const FILE: &str = "rice-theme.json";
const MAX_FILE_BYTES: u64 = 16 * 1024;
const PROFILE_BUDGET: Duration = Duration::from_secs(60);
const MAX_TEXTURE_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Default, Deserialize)]
struct RiceTheme {
    accent: Option<String>,
    /// An absolute PNG path; `""` clears the texture.
    texture: Option<PathBuf>,
    texture_opacity: Option<u8>,
    radius: Option<u8>,
    toolbar: Option<String>,
    window_controls: Option<String>,
    /// Fields this build does not know, kept only to name them in the log.
    #[serde(flatten)]
    unknown: BTreeMap<String, serde_json::Value>,
}

/// The last texture file read: its path, modification time and length, and
/// the digest of its bytes. The same file is not read or hashed again.
pub(crate) struct TextureMemo {
    path: PathBuf,
    modified: SystemTime,
    len: u64,
    digest: String,
}

fn read(path: &Path) -> Option<RiceTheme> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        crate::write_diagnostic(format_args!("rice-theme: not a small file, ignored"));
        return None;
    }
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            crate::write_diagnostic(format_args!("rice-theme: unreadable, ignored: {error}"));
            return None;
        }
    };
    match serde_json::from_str::<RiceTheme>(&text) {
        Ok(theme) => {
            if !theme.unknown.is_empty() {
                let names: Vec<&str> = theme.unknown.keys().map(String::as_str).collect();
                crate::write_diagnostic(format_args!(
                    "rice-theme: unknown fields ignored: {}",
                    names.join(", ")
                ));
            }
            Some(theme)
        }
        Err(error) => {
            crate::write_diagnostic(format_args!("rice-theme: malformed, ignored: {error}"));
            None
        }
    }
}

/// `#rrggbb` to the custom theme the core accepts, built from the stored
/// theme so only its accent changes.
fn accent_theme(hex: &str) -> Option<(String, String)> {
    let hex = hex.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    let rgb = [channel(0)?, channel(2)?, channel(4)?];
    let theme =
        crate::app_setting("ui.theme").unwrap_or_else(|| zephium_core::theme::DEFAULT.to_owned());
    let custom = crate::app_setting("ui.theme-custom");
    // One accent for both appearances: rice is monochrome and dark-first.
    let carried = zephium_core::theme::with_accent(&theme, custom.as_deref(), (rgb, rgb))?;
    Some((zephium_core::theme::CUSTOM.to_owned(), carried))
}

fn set(shell: &Handle, key: &str, value: String) {
    if !zephium_core::preferences::value_allowed(key, &value) {
        crate::write_diagnostic(format_args!("rice-theme: {key} refused"));
        return;
    }
    if crate::app_setting(key).as_deref() == Some(value.as_str()) {
        return;
    }
    if !shell.dispatch(Command::SetAppSetting {
        key: key.to_owned(),
        value,
    }) {
        crate::write_diagnostic(format_args!(
            "rice-theme: {key} not applied, shell refused it"
        ));
    }
}

/// The current profile, once the shell is admitted. At launch the shell is
/// still suspended, so each request may sit queued; ask again until the budget
/// is spent.
fn current_profile(shell: &Handle) -> Option<zephium_core::ids::ProfileId> {
    let deadline = std::time::Instant::now() + PROFILE_BUDGET;
    loop {
        let reply = shell.profiles().recv_timeout(Duration::from_secs(8));
        if let Some(current) = reply.ok().flatten().and_then(|view| view.current) {
            return zephium_core::ids::ProfileId::parse(&current);
        }
        if std::time::Instant::now() >= deadline {
            return None;
        }
    }
}

/// The digest the store gives these bytes: lowercase hex SHA-256, as
/// `zephium_store::hub::media::admit` computes it.
fn media_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn texture_digest(shell: &Handle, path: &Path, memo: &mut Option<TextureMemo>) -> Option<String> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_file() && metadata.len() <= MAX_TEXTURE_BYTES => metadata,
        Ok(_) => {
            crate::write_diagnostic(format_args!("rice-theme: texture is not a file or too big"));
            return None;
        }
        Err(error) => {
            crate::write_diagnostic(format_args!("rice-theme: texture missing: {error}"));
            return None;
        }
    };
    let stored = crate::app_setting("ui.texture");
    let modified = metadata.modified().ok();
    // The same file as last time, already the stored texture: nothing to do.
    if let (Some(known), Some(modified)) = (memo.as_ref(), modified) {
        if known.path == path
            && known.modified == modified
            && known.len == metadata.len()
            && stored.as_deref() == Some(known.digest.as_str())
        {
            return Some(known.digest.clone());
        }
    }
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            crate::write_diagnostic(format_args!("rice-theme: texture unreadable: {error}"));
            return None;
        }
    };
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        crate::write_diagnostic(format_args!("rice-theme: texture is not a PNG"));
        return None;
    }
    let digest = media_digest(&bytes);
    if let Some(modified) = modified {
        *memo = Some(TextureMemo {
            path: path.to_owned(),
            modified,
            len: metadata.len(),
            digest: digest.clone(),
        });
    }
    // The store already holds these bytes as the texture; importing them
    // again would only add another resource row.
    if stored.as_deref() == Some(digest.as_str()) {
        return Some(digest);
    }
    let Some(profile) = current_profile(shell) else {
        crate::write_diagnostic(format_args!(
            "rice-theme: no profile for the texture in time"
        ));
        return None;
    };
    let receiver = shell.import_media(
        profile,
        zephium_core::resources::MediaImport {
            request_id: format!("rice-texture-{}", zephium_core::ids::ResourceId::generate()),
            name: "rice-texture.png".into(),
            origin: zephium_core::resources::MediaOrigin::Imported,
            bytes: std::sync::Arc::new(bytes),
        },
    );
    let Ok(reply) = receiver.recv_timeout(Duration::from_secs(30)) else {
        crate::write_diagnostic(format_args!("rice-theme: texture import timed out"));
        return None;
    };
    match reply.response {
        zephium_core::resources::ResourceResponse::Applied { record, .. } => {
            match record.draft.content {
                zephium_core::resources::ResourceContent::Media { asset } => Some(asset.digest),
                _ => {
                    crate::write_diagnostic(format_args!(
                        "rice-theme: texture import was not media"
                    ));
                    None
                }
            }
        }
        _ => {
            crate::write_diagnostic(format_args!("rice-theme: texture import refused"));
            None
        }
    }
}

fn apply(shell: &Handle, path: &Path, memo: &mut Option<TextureMemo>) {
    let Some(theme) = read(path) else {
        return;
    };
    if let Some((name, custom)) = theme.accent.as_deref().and_then(accent_theme) {
        set(shell, "ui.theme-custom", custom);
        set(shell, "ui.theme", name);
    }
    if let Some(opacity) = theme.texture_opacity {
        set(shell, "ui.texture-opacity", opacity.to_string());
    }
    if let Some(radius) = theme.radius {
        set(shell, "ui.radius", radius.to_string());
    }
    if let Some(toolbar) = theme.toolbar {
        set(shell, "ui.toolbar", toolbar);
    }
    if let Some(controls) = theme.window_controls {
        set(shell, "ui.window-controls", controls);
    }
    if let Some(texture) = theme.texture.as_deref() {
        if texture.as_os_str().is_empty() {
            set(shell, "ui.texture", String::new());
        } else if let Some(digest) = texture_digest(shell, texture, memo) {
            set(shell, "ui.texture", digest);
        }
    }
}

/// Applies the file once and then on each change, off the main thread.
pub(crate) fn start(dir: PathBuf, shell: Handle) {
    std::thread::Builder::new()
        .name("rice-theme".into())
        .spawn(move || {
            let path = dir.join(FILE);
            let mut memo = None;
            apply(&shell, &path, &mut memo);
            let (sender, events) = std::sync::mpsc::channel();
            let mut watcher = match notify::recommended_watcher(sender) {
                Ok(watcher) => watcher,
                Err(error) => {
                    crate::write_diagnostic(format_args!("rice-theme: no file watcher: {error}"));
                    return;
                }
            };
            use notify::Watcher;
            // The directory, not the file: rice replaces the file by rename.
            if let Err(error) = watcher.watch(&dir, notify::RecursiveMode::NonRecursive) {
                crate::write_diagnostic(format_args!("rice-theme: cannot watch {dir:?}: {error}"));
                return;
            }
            while let Ok(event) = events.recv() {
                let Ok(event) = event else { continue };
                if event
                    .paths
                    .iter()
                    .any(|p| p.file_name() == Some(FILE.as_ref()))
                {
                    // Let a burst of writes from one apply settle first.
                    std::thread::sleep(Duration::from_millis(150));
                    while events.try_recv().is_ok() {}
                    apply(&shell, &path, &mut memo);
                }
            }
        })
        .ok();
}
