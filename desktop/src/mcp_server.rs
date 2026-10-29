//! rice fork: the desktop's side of the MCP server — the Backend over Ask's
//! own page and tab adapters, consent prompts, the audit log and lifecycle.
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{
    DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult,
};
use tokio::sync::watch;
use zephium_app::{Command, Handle};
use zephium_core::ids::ItemId;
use zephium_core::ports::store::Store;
use zephium_mcp_server::{
    Backend, BoxFuture, HistoryEntry, McpServer, PageRead, Passage, Refusal, StepIn, TabInfo,
};
use zephium_work_composition::page_assist::core::actions::{Direction, Verb};
use zephium_work_composition::page_assist::core::prompt::fence;
use zephium_work_composition::page_assist::port::{
    FieldKind, ReadFailure, Role, StepFailure, StepRequest,
};

use crate::page_assist::{mcp_active, mcp_ports, mcp_reader, mcp_tabs};

const MAX_AUDIT_BYTES: u64 = 1024 * 1024;
/// The longest page text a prompt shows, in characters.
const MAX_PROMPT_TEXT: usize = 160;

const ALLOW: &str = "Allow";
const DENY: &str = "Deny";
const ALLOW_ONCE: &str = "Allow once";
const ALLOW_ALWAYS: &str = "Allow always";

static SERVER: Mutex<Option<McpServer>> = Mutex::new(None);
/// The person's latest choice; one task follows it, so starts and stops never overlap.
static WANT: OnceLock<watch::Sender<bool>> = OnceLock::new();
/// Bumped on every start and stop, so a stopped server's last client list is not shown.
static GENERATION: AtomicU64 = AtomicU64::new(0);
/// One audit writer at a time, so rotation and appends do not interleave.
static AUDIT: Mutex<()> = Mutex::new(());

struct DesktopBackend {
    app: AppHandle,
    dir: PathBuf,
    generation: u64,
}

fn read_failure(failure: ReadFailure) -> Refusal {
    match failure {
        ReadFailure::NotReady => Refusal::NotReady,
        ReadFailure::PageChanged => Refusal::PageChanged,
        ReadFailure::Unavailable => Refusal::Failed,
    }
}

fn step_failure(failure: StepFailure) -> Refusal {
    match failure {
        StepFailure::PageChanged => Refusal::PageChanged,
        StepFailure::Unsuitable => {
            Refusal::Invalid("That passage is not a control this step can use.")
        }
        StepFailure::Refused => Refusal::Sensitive,
        StepFailure::Failed => Refusal::Failed,
    }
}

fn role_name(role: Role) -> String {
    match role {
        Role::Heading(level) => format!("heading{level}"),
        Role::Text => "text".into(),
        Role::ListItem { .. } => "list_item".into(),
        Role::Cell => "cell".into(),
        Role::Link { .. } => "link".into(),
        Role::Button => "button".into(),
        Role::Field(kind) => match kind {
            FieldKind::Text => "field",
            FieldKind::Search => "search_field",
            FieldKind::Email => "email_field",
            FieldKind::Number => "number_field",
            FieldKind::Password => "password_field",
            FieldKind::Card => "card_field",
            FieldKind::OneTimeCode => "code_field",
        }
        .into(),
        Role::Checkbox { checked: true } => "checkbox_checked".into(),
        Role::Checkbox { checked: false } => "checkbox".into(),
        Role::Select => "select".into(),
        Role::Other => "other".into(),
    }
}

/// Untrusted text for a native prompt: one line, bounded.
fn prompt_text(text: &str) -> String {
    let mut line: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(MAX_PROMPT_TEXT + 1)
        .collect();
    if line.chars().count() > MAX_PROMPT_TEXT {
        line = line.chars().take(MAX_PROMPT_TEXT).collect::<String>() + "…";
    }
    line
}

/// A client-supplied name for one audit field: bounded, no control characters.
fn audit_field(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).take(64).collect()
}

impl DesktopBackend {
    fn shell(&self) -> Option<tauri::State<'_, Handle>> {
        self.app.try_state::<Handle>()
    }

    /// The epoch of a tab outside private windows and sensitive sites.
    /// A tab with no site (an internal page) is refused like the reader does.
    fn readable(&self, tab: &str) -> Result<u64, Refusal> {
        let (page, _) = mcp_ports().ok_or(Refusal::Failed)?;
        let facts = page.facts(tab).ok_or(Refusal::NotFound)?;
        if facts.private {
            return Err(Refusal::Private);
        }
        let site = facts
            .url
            .as_deref()
            .and_then(zephium_app::work_sites::site_of);
        if site.is_none_or(|site| zephium_app::work_sites::is_sensitive(&site)) {
            return Err(Refusal::Sensitive);
        }
        if facts.loading {
            return Err(Refusal::NotReady);
        }
        Ok(facts.epoch)
    }

    fn step_request(step: &StepIn) -> Result<StepRequest, Refusal> {
        Ok(StepRequest {
            verb: Verb::parse(&step.verb).ok_or(Refusal::Invalid("Unknown verb."))?,
            read: step.read,
            target: step.target,
            direction: match step.direction.as_deref() {
                Some("up") => Some(Direction::Up),
                Some("down") => Some(Direction::Down),
                _ => None,
            },
            value: step.value.clone(),
        })
    }

    fn listed(&self, tab: &str) -> Result<(), Refusal> {
        mcp_tabs()
            .iter()
            .any(|(id, ..)| id == tab)
            .then_some(())
            .ok_or(Refusal::NotFound)
    }

    /// Shows a native prompt and waits for its answer. The prompt belongs to
    /// the call that asked: when that call ends first (deadline, the
    /// client's cancel, a stop) this future is dropped, the answer has
    /// nowhere to go, and nothing acts on it. The dialog cannot be closed
    /// from here, so a late approval says so instead of seeming to work.
    async fn ask(
        &self,
        title: &str,
        body: String,
        buttons: MessageDialogButtons,
        client: &str,
    ) -> Option<MessageDialogResult> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let app = self.app.clone();
        let client = client.to_owned();
        self.app
            .dialog()
            .message(body)
            .title(title)
            .kind(MessageDialogKind::Warning)
            .buttons(buttons)
            .show_with_result(move |result| {
                // `send` fails exactly when the asking call has ended.
                let Err(late) = sender.send(result) else {
                    return;
                };
                if matches!(&late, MessageDialogResult::Custom(label) if label != DENY) {
                    app.dialog()
                        .message(format!(
                            "{client}'s request had already ended, so nothing was done. It can ask again."
                        ))
                        .title("Zephium")
                        .kind(MessageDialogKind::Info)
                        .buttons(MessageDialogButtons::Ok)
                        .show(|_| {});
                }
            });
        receiver.await.ok()
    }

    /// Adds `client` to the always-allowed list, when it still fits.
    fn remember(&self, allowed: &str, client: &str) {
        let mut names = zephium_core::preferences::mcp_allowed(allowed);
        if names.contains(&client) || names.len() >= zephium_core::preferences::MAX_MCP_CLIENTS {
            return;
        }
        names.push(client);
        let value = names.join("\n");
        if zephium_core::preferences::value_allowed("mcp.allowed", &value) {
            if let Some(shell) = self.shell() {
                shell.dispatch(Command::SetAppSetting {
                    key: "mcp.allowed".into(),
                    value,
                });
            }
        }
    }
}

impl Backend for DesktopBackend {
    fn list_tabs(&self) -> Vec<TabInfo> {
        let (_, active, private) = mcp_active();
        mcp_tabs()
            .into_iter()
            .map(|(id, title, url, space)| TabInfo {
                active: !private && active.as_deref() == Some(id.as_str()),
                id,
                title,
                url,
                space: Some(space),
            })
            .collect()
    }

    fn read_page<'a>(&'a self, tab: &'a str) -> BoxFuture<'a, Result<PageRead, Refusal>> {
        Box::pin(async move {
            let epoch = self.readable(tab)?;
            let (page, _) = mcp_ports().ok_or(Refusal::Failed)?;
            let readout = page.read(tab, epoch).await.map_err(read_failure)?;
            Ok(PageRead {
                read: readout.read,
                title: readout.title.map(|title| fence(&title)),
                passages: readout
                    .passages
                    .into_iter()
                    .map(|passage| Passage {
                        id: passage.id,
                        role: role_name(passage.role),
                        text: fence(&passage.text),
                        href: passage.href,
                    })
                    .collect(),
            })
        })
    }

    fn search_history(&self, query: &str, limit: u32) -> Result<Vec<HistoryEntry>, Refusal> {
        let (space, _, private) = mcp_active();
        if private {
            return Err(Refusal::Private);
        }
        let (_, tabs) = mcp_ports().ok_or(Refusal::Failed)?;
        let profile = space
            .as_deref()
            .and_then(|space| tabs.profile(space))
            .ok_or(Refusal::Failed)?;
        let store = crate::app_store().ok_or(Refusal::Failed)?;
        Ok(store
            .search_history(profile, query, limit)
            .into_iter()
            .map(|hit| HistoryEntry {
                url: hit.url,
                title: fence(&hit.title),
                last_visit: hit.last_visit,
            })
            .collect())
    }

    fn open_tab<'a>(
        &'a self,
        url: &'a str,
        background: bool,
    ) -> BoxFuture<'a, Result<String, Refusal>> {
        Box::pin(async move {
            let (space, _, private) = mcp_active();
            if private {
                return Err(Refusal::Private);
            }
            let space = space.ok_or(Refusal::Failed)?;
            let (_, tabs) = mcp_ports().ok_or(Refusal::Failed)?;
            let profile = tabs.profile(&space).ok_or(Refusal::Failed)?;
            tabs.open(&profile, &space, url, None, !background)
                .await
                .ok_or(Refusal::Failed)
        })
    }

    fn navigate(&self, tab: &str, url: &str) -> Result<(), Refusal> {
        let (page, tabs) = mcp_ports().ok_or(Refusal::Failed)?;
        let facts = page.facts(tab).ok_or(Refusal::NotFound)?;
        if facts.private {
            return Err(Refusal::Private);
        }
        tabs.navigate(&facts.profile, tab, url)
            .then_some(())
            .ok_or(Refusal::Failed)
    }

    fn activate(&self, tab: &str) -> Result<(), Refusal> {
        self.listed(tab)?;
        let item = ItemId::parse(tab).ok_or(Refusal::NotFound)?;
        self.shell()
            .ok_or(Refusal::Failed)?
            .dispatch(Command::Activate(item))
            .then_some(())
            .ok_or(Refusal::Failed)
    }

    fn close(&self, tab: &str) -> Result<(), Refusal> {
        self.listed(tab)?;
        let item = ItemId::parse(tab).ok_or(Refusal::NotFound)?;
        self.shell()
            .ok_or(Refusal::Failed)?
            .dispatch(Command::Close(item))
            .then_some(())
            .ok_or(Refusal::Failed)
    }

    fn resolve_step(&self, step: &StepIn) -> Result<String, Refusal> {
        let epoch = self.readable(&step.tab)?;
        let (page, _) = mcp_ports().ok_or(Refusal::Failed)?;
        let resolved = page
            .resolve(&step.tab, epoch, &Self::step_request(step)?)
            .map_err(step_failure)?;
        Ok(resolved.label)
    }

    fn run_step<'a>(&'a self, step: &'a StepIn) -> BoxFuture<'a, Result<(), Refusal>> {
        Box::pin(async move {
            let epoch = self.readable(&step.tab)?;
            let (page, _) = mcp_ports().ok_or(Refusal::Failed)?;
            let request = Self::step_request(step)?;
            page.act(&step.tab, epoch, &request)
                .await
                .map_err(step_failure)
        })
    }

    fn confirm<'a>(&'a self, client: &'a str, action: String) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            let answer = self
                .ask(
                    "Zephium",
                    format!("{client} wants to {}.", prompt_text(&action)),
                    MessageDialogButtons::OkCancelCustom(ALLOW.into(), DENY.into()),
                    client,
                )
                .await;
            matches!(answer, Some(MessageDialogResult::Custom(label)) if label == ALLOW)
        })
    }

    fn admit<'a>(&'a self, client: &'a str) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            let allowed = crate::app_setting("mcp.allowed").unwrap_or_default();
            if zephium_core::preferences::mcp_allowed(&allowed).contains(&client) {
                return true;
            }
            let answer = self
                .ask(
                    &format!("Allow {client} to use Zephium?"),
                    format!(
                        "{client} wants to see your tabs, read pages and open sites in Zephium. \
                         Closing tabs and page steps will still ask you each time."
                    ),
                    // The default (first) button is the one that grants least.
                    MessageDialogButtons::YesNoCancelCustom(
                        ALLOW_ONCE.into(),
                        ALLOW_ALWAYS.into(),
                        DENY.into(),
                    ),
                    client,
                )
                .await;
            match answer {
                Some(MessageDialogResult::Custom(label)) if label == ALLOW_ALWAYS => {
                    // Read again: another prompt may have added a name meanwhile.
                    let allowed = crate::app_setting("mcp.allowed").unwrap_or_default();
                    self.remember(&allowed, client);
                    true
                }
                Some(MessageDialogResult::Custom(label)) => label == ALLOW_ONCE,
                _ => false,
            }
        })
    }

    fn connected(&self, clients: &[String]) {
        if self.generation != GENERATION.load(Ordering::SeqCst) {
            return;
        }
        crate::emit_ui_command(
            &self.app,
            &format!("mcp.clients={}", clients.join("\u{1f}")),
        );
    }

    fn audit(&self, client: &str, tool: &str, tab: Option<&str>, outcome: &str) {
        let _writer = AUDIT.lock().unwrap_or_else(|p| p.into_inner());
        let path = self.dir.join("mcp-audit.log");
        if std::fs::metadata(&path).is_ok_and(|meta| meta.len() > MAX_AUDIT_BYTES) {
            let _ = std::fs::rename(&path, self.dir.join("mcp-audit.log.1"));
        }
        let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(&path)
        else {
            return;
        };
        let at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        let line = serde_json::json!({
            "at": at,
            "client": client,
            "tool": audit_field(tool),
            "tab": tab.map(audit_field),
            "outcome": outcome,
        });
        let _ = writeln!(file, "{line}");
    }
}

/// `<app data dir>/mcp`: the socket and the audit log. The server refuses a
/// socket whose directory has any other name.
fn mcp_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|dir| dir.join("mcp"))
}

/// Stops the running server now: its clients are dropped and the socket
/// file is gone before this returns.
fn stop_now() {
    let server = SERVER.lock().unwrap_or_else(|p| p.into_inner()).take();
    GENERATION.fetch_add(1, Ordering::SeqCst);
    if let Some(server) = server {
        server.stop();
    }
}

/// Follows the person's choice, one start at a time.
async fn follow(app: AppHandle, dir: PathBuf, mut want: watch::Receiver<bool>) {
    loop {
        let enabled = *want.borrow_and_update();
        let running = SERVER.lock().unwrap_or_else(|p| p.into_inner()).is_some();
        if enabled && !running {
            let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
            let backend = Arc::new(DesktopBackend {
                app: app.clone(),
                dir: dir.clone(),
                generation,
            });
            match McpServer::start(dir.join("mcp.sock"), backend).await {
                Ok(server) => {
                    let mut slot = SERVER.lock().unwrap_or_else(|p| p.into_inner());
                    // Turned off while starting: it never serves.
                    if *want.borrow() && GENERATION.load(Ordering::SeqCst) == generation {
                        *slot = Some(server);
                    } else {
                        server.stop();
                    }
                }
                Err(error) => {
                    crate::write_diagnostic(format_args!("mcp: socket unavailable: {error}"));
                    crate::emit_ui_command(&app, "mcp.error=socket");
                }
            }
        }
        if want.changed().await.is_err() {
            return;
        }
    }
}

/// Starts or stops the server to match `mcp.enabled`. The page reader stays
/// on while the server is on, even with AI off.
pub(crate) fn set_enabled(app: &AppHandle, enabled: bool) {
    let Some(dir) = mcp_dir(app) else {
        return;
    };
    let was = match WANT.get() {
        Some(want) => want.send_replace(enabled),
        None if !enabled => false,
        None => {
            let (sender, receiver) = watch::channel(true);
            if WANT.set(sender).is_err() {
                // Another caller won the race; follow its task.
                return set_enabled(app, enabled);
            }
            tauri::async_runtime::spawn(follow(app.clone(), dir, receiver));
            false
        }
    };
    if was == enabled {
        return;
    }
    if !enabled {
        stop_now();
    }
    mcp_reader(enabled);
    crate::emit_ui_command(app, "mcp.clients=");
}

/// On app exit: removes the socket without touching the reader or the UI.
pub(crate) fn shutdown() {
    if let Some(want) = WANT.get() {
        want.send_replace(false);
    }
    stop_now();
}
