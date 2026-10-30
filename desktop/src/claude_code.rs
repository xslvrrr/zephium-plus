//! rice fork: Ask answered by Claude Code.
//!
//! With `ai.claude-code` on, the Ask panel's prompts run as headless Claude
//! Code tasks (`claude -p`), one process per prompt, continuing the same
//! Claude Code session across a conversation. Claude Code reaches the
//! browser only through Zephium's own MCP server (`zephium-mcp`, the bridge
//! bundled beside the app), whose consent prompts and per-client grants
//! still apply; no other MCP server is loaded, and Claude Code's own tools
//! that would act on the Mac are not allowed. Its output streams to the
//! panel as `zephium:claude-code` events.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Manager};

const EVENT: &str = "zephium:claude-code";
const MAX_PROMPT: usize = 32 * 1024;
const MAX_LINE: usize = 4 * 1024 * 1024;
const MAX_TEXT: usize = 256 * 1024;
const STDERR_TAIL: usize = 2 * 1024;

/// What Claude Code may use: Zephium's tools, and reading the web.
const ALLOWED: &[&str] = &["mcp__zephium", "WebSearch", "WebFetch"];

const SYSTEM: &str = "You are answering inside the Zephium web browser, from its Ask panel. \
Act on the browser only through the `zephium` MCP tools (list tabs, read a page, open, \
navigate, activate or close a tab, search history, click or fill on a page); the person \
approves each action in Zephium. Answer in short Markdown. Do not try to edit files or run \
commands on the Mac.";

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) mod model;

static NEXT: AtomicU32 = AtomicU32::new(1);

fn runs() -> &'static Mutex<HashMap<u32, Arc<Mutex<Child>>>> {
    static RUNS: OnceLock<Mutex<HashMap<u32, Arc<Mutex<Child>>>>> = OnceLock::new();
    RUNS.get_or_init(Default::default)
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeCodeStatus {
    /// Where `claude` was found; `None` when it is not installed.
    pub path: Option<String>,
    /// Zephium's MCP bridge is bundled beside the app.
    pub bridge: bool,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ClaudeCodeEvent {
    /// Claude Code started, in this session.
    Started { run: u32, session: String },
    /// A piece of the answer, in order.
    Text { run: u32, text: String },
    /// Claude Code called a tool.
    Tool { run: u32, name: String },
    /// The run ended; `error` says why when it failed.
    Done {
        run: u32,
        session: Option<String>,
        error: Option<String>,
        cost: Option<f64>,
    },
}

/// `claude` where the person's shell would find it; an app opened from the
/// Finder has no login `PATH`.
fn locate() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(home) = &home {
        candidates.push(home.join(".local/bin/claude"));
        candidates.push(home.join(".claude/local/claude"));
        candidates.push(home.join(".npm-global/bin/claude"));
        candidates.push(home.join(".bun/bin/claude"));
    }
    candidates.push(PathBuf::from("/opt/homebrew/bin/claude"));
    candidates.push(PathBuf::from("/usr/local/bin/claude"));
    if let Some(path) = std::env::var_os("PATH") {
        candidates.extend(std::env::split_paths(&path).map(|dir| dir.join("claude")));
    }
    candidates.into_iter().find(|path| executable(path))
}

#[cfg(unix)]
fn executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn executable(path: &Path) -> bool {
    path.is_file()
}

fn bridge() -> Option<PathBuf> {
    let path = std::env::current_exe().ok()?.with_file_name("zephium-mcp");
    executable(&path).then_some(path)
}

pub(crate) fn status() -> ClaudeCodeStatus {
    ClaudeCodeStatus {
        path: locate().map(|path| path.display().to_string()),
        bridge: bridge().is_some(),
    }
}

/// A `PATH` with the usual places a Node or native install keeps its tools.
fn search_path() -> String {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        dirs.push(home.join(".local/bin"));
        dirs.push(home.join(".bun/bin"));
    }
    for dir in [
        "/opt/homebrew/bin",
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
        "/usr/sbin",
        "/sbin",
    ] {
        dirs.push(PathBuf::from(dir));
    }
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    std::env::join_paths(dirs)
        .map(|joined| joined.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn clip(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_owned();
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

/// Starts one headless run for `prompt`, continuing `session` when given.
/// Returns the run's number; its output arrives as events to `label`.
pub(crate) fn ask(
    app: &AppHandle,
    label: String,
    prompt: String,
    session: Option<String>,
    page: Option<String>,
) -> Result<u32, String> {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err("Ask something first.".to_owned());
    }
    if prompt.len() > MAX_PROMPT {
        return Err("That is too long to ask.".to_owned());
    }
    if crate::app_setting("mcp.enabled").as_deref() != Some("true") {
        return Err("Turn on Claude access in Settings › Connections first.".to_owned());
    }
    let claude = locate().ok_or("Claude Code is not installed (no `claude` found).")?;
    let bridge = bridge().ok_or("Zephium's MCP bridge is missing from this build.")?;
    let config = serde_json::json!({
        "mcpServers": { "zephium": { "type": "stdio", "command": bridge, "args": [] } }
    })
    .to_string();
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("claude-code");
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let system = match page
        .as_deref()
        .map(str::trim)
        .filter(|page| !page.is_empty())
    {
        Some(page) => format!("{SYSTEM}\n\nThe tab in front: {}", clip(page, 2048)),
        None => SYSTEM.to_owned(),
    };

    let mut command = Command::new(&claude);
    command
        .arg("-p")
        .arg(prompt)
        .args(["--output-format", "stream-json", "--verbose"])
        .arg("--mcp-config")
        .arg(&config)
        .arg("--strict-mcp-config")
        .arg("--allowedTools")
        .args(ALLOWED)
        .arg("--append-system-prompt")
        .arg(&system);
    if let Some(session) = session.filter(|session| {
        !session.is_empty()
            && session.len() <= 128
            && session
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    }) {
        command.arg("--resume").arg(session);
    }
    command
        .current_dir(&dir)
        .env("PATH", search_path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("Claude Code did not start: {error}"))?;
    let stdout = child.stdout.take().ok_or("Claude Code gave no output.")?;
    let stderr = child.stderr.take();
    let run = NEXT.fetch_add(1, Ordering::Relaxed);
    let child = Arc::new(Mutex::new(child));
    runs()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(run, child.clone());

    let tail = Arc::new(Mutex::new(String::new()));
    if let Some(stderr) = stderr {
        let tail = tail.clone();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut chunk = [0u8; 1024];
            while let Ok(read) = reader.read(&mut chunk) {
                if read == 0 {
                    break;
                }
                let mut tail = tail.lock().unwrap_or_else(|e| e.into_inner());
                tail.push_str(&String::from_utf8_lossy(&chunk[..read]));
                if tail.len() > STDERR_TAIL {
                    let mut cut = tail.len() - STDERR_TAIL;
                    while !tail.is_char_boundary(cut) {
                        cut += 1;
                    }
                    tail.drain(..cut);
                }
            }
        });
    }

    let app = app.clone();
    std::thread::spawn(move || {
        let emit = |event: ClaudeCodeEvent| crate::emit_to_privileged(&app, &label, EVENT, &event);
        let mut session = None;
        let mut finished = false;
        let mut sent = 0usize;
        let mut reader = BufReader::new(stdout);
        let mut line = Vec::new();
        loop {
            line.clear();
            match reader
                .by_ref()
                .take(MAX_LINE as u64)
                .read_until(b'\n', &mut line)
            {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            let Ok(value) = serde_json::from_slice::<Value>(&line) else {
                continue;
            };
            match value.get("type").and_then(Value::as_str) {
                Some("system") => {
                    if let Some(id) = value.get("session_id").and_then(Value::as_str) {
                        if session.is_none() {
                            session = Some(id.to_owned());
                            emit(ClaudeCodeEvent::Started {
                                run,
                                session: id.to_owned(),
                            });
                        }
                    }
                }
                Some("assistant") => {
                    let blocks = value
                        .pointer("/message/content")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    for block in blocks {
                        match block.get("type").and_then(Value::as_str) {
                            Some("text") => {
                                let text = block.get("text").and_then(Value::as_str).unwrap_or("");
                                if !text.is_empty() && sent < MAX_TEXT {
                                    sent += text.len();
                                    emit(ClaudeCodeEvent::Text {
                                        run,
                                        text: clip(text, MAX_TEXT),
                                    });
                                }
                            }
                            Some("tool_use") => {
                                let name = block.get("name").and_then(Value::as_str).unwrap_or("");
                                emit(ClaudeCodeEvent::Tool {
                                    run,
                                    name: clip(name, 128),
                                });
                            }
                            _ => {}
                        }
                    }
                }
                Some("result") => {
                    finished = true;
                    let failed = value
                        .get("is_error")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let error = failed.then(|| {
                        value
                            .get("result")
                            .and_then(Value::as_str)
                            .map(|text| clip(text, 2048))
                            .unwrap_or_else(|| "Claude Code could not answer.".to_owned())
                    });
                    emit(ClaudeCodeEvent::Done {
                        run,
                        session: value
                            .get("session_id")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                            .or_else(|| session.clone()),
                        error,
                        cost: value.get("total_cost_usd").and_then(Value::as_f64),
                    });
                }
                _ => {}
            }
        }
        let status = child.lock().unwrap_or_else(|e| e.into_inner()).wait();
        runs()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&run);
        if !finished {
            let tail = tail
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .trim()
                .to_owned();
            let error = match status {
                Ok(status) if status.success() => "Claude Code ended without an answer.".to_owned(),
                _ if !tail.is_empty() => clip(&tail, 2048),
                Ok(_) => "Claude Code stopped.".to_owned(),
                Err(error) => error.to_string(),
            };
            emit(ClaudeCodeEvent::Done {
                run,
                session,
                error: Some(error),
                cost: None,
            });
        }
    });
    Ok(run)
}

/// Stops a run; its reader reports it as done.
pub(crate) fn cancel(run: u32) {
    let child = runs()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&run)
        .cloned();
    if let Some(child) = child {
        let _ = child.lock().unwrap_or_else(|e| e.into_inner()).kill();
    }
}
