//! rice fork: the desktop's side of the MCP server — the Backend over Ask's
//! own page and tab adapters, consent prompts, the audit log and lifecycle.
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use rfd::{AsyncMessageDialog, MessageButtons, MessageDialogResult, MessageLevel};
use tauri::{AppHandle, Manager};
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

use crate::page_assist::{mcp_active, mcp_outside_input, mcp_ports, mcp_reader, mcp_tabs};

const MAX_AUDIT_BYTES: u64 = 1024 * 1024;
/// The longest page text a prompt shows, in characters.
const MAX_PROMPT_TEXT: usize = 160;
/// The longest fill value a prompt shows, in characters.
const MAX_PROMPT_VALUE: usize = 200;
/// The longest action a confirm shows: a page label and a fill value fit.
const MAX_PROMPT_ACTION: usize = 400;
/// `read_page`'s bound (spec §2.4): passages, and bytes of text plus links.
const MAX_READ_PASSAGES: usize = 256;
const MAX_READ_BYTES: usize = 48 * 1024;
/// Consent prompts open at once, across all connections.
const MAX_OPEN_PROMPTS: usize = 3;

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
/// The connections with a consent prompt on screen, one entry per prompt.
static PROMPTS: Mutex<Vec<u64>> = Mutex::new(Vec::new());

/// The person's answer to a consent prompt. Only a press of an Allow button
/// is an Allow answer: a dialog that failed, was cancelled, closed with
/// Escape or never answered is `Deny`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    Deny,
    Allow,
    AllowOnce,
    AllowAlways,
}

impl Answer {
    /// rfd's raw result, strictly: only `Custom` with an Allow button's exact
    /// label grants. rfd reports a failed alert, a lost alert thread and a
    /// dismissal (Escape, GTK's close button) as `Cancel`, and GTK reports
    /// custom buttons as `Ok`/`Cancel`/`Yes`/`No`; all of those deny. (Not
    /// tauri-plugin-dialog's result: it turns `Cancel` into the last button's
    /// label, which here is an Allow.)
    fn from_dialog(result: MessageDialogResult) -> Self {
        let MessageDialogResult::Custom(label) = result else {
            return Self::Deny;
        };
        match label.as_str() {
            ALLOW => Self::Allow,
            ALLOW_ONCE => Self::AllowOnce,
            ALLOW_ALWAYS => Self::AllowAlways,
            _ => Self::Deny,
        }
    }
}

/// Shows `dialog` as tauri-plugin-dialog does: built and shown from the main
/// thread, its result awaited on the async runtime. When the dialog cannot be
/// shown, `answered` is dropped without being called.
fn show_dialog(
    app: &AppHandle,
    dialog: AsyncMessageDialog,
    answered: impl FnOnce(MessageDialogResult) + Send + 'static,
) {
    let _ = app.run_on_main_thread(move || {
        let result = dialog.show();
        tauri::async_runtime::spawn(async move { answered(result.await) });
    });
}

/// A consent prompt's place in `PROMPTS`, held until the person answers it,
/// not until the asking call ends: a prompt outlives a call that timed out,
/// and it still counts.
struct PromptSlot(u64);

impl PromptSlot {
    /// One prompt per connection and `MAX_OPEN_PROMPTS` overall, or `Busy`.
    fn take(connection: u64) -> Result<Self, Refusal> {
        let mut open = PROMPTS.lock().unwrap_or_else(|p| p.into_inner());
        if open.contains(&connection) || open.len() >= MAX_OPEN_PROMPTS {
            return Err(Refusal::Busy);
        }
        open.push(connection);
        Ok(Self(connection))
    }
}

impl Drop for PromptSlot {
    fn drop(&mut self) {
        let mut open = PROMPTS.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(at) = open.iter().position(|&connection| connection == self.0) {
            open.swap_remove(at);
        }
    }
}

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

/// Untrusted text for a native prompt: one line, at most `max` characters.
fn prompt_text(text: &str, max: usize) -> String {
    let mut line: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(max + 1)
        .collect();
    if line.chars().count() > max {
        line = line.chars().take(max).collect::<String>() + "…";
    }
    line
}

/// A fill's confirm names the value: `Type “value” into “Search”`. The
/// planner's label is `Type into "Search"`.
fn fill_label(label: &str, value: &str) -> String {
    // The value's own quotes become `'`, so it cannot close the “…” early
    // and imitate the confirm's wording.
    let value = prompt_text(value, MAX_PROMPT_VALUE).replace(['“', '”', '"'], "'");
    let target = label.strip_prefix("Type into ").map(|target| {
        target
            .strip_prefix('"')
            .and_then(|target| target.strip_suffix('"'))
            .unwrap_or(target)
    });
    match target {
        Some(target) => format!(
            "Type “{value}” into “{}”",
            prompt_text(target, MAX_PROMPT_TEXT)
        ),
        None => format!("{}: “{value}”", prompt_text(label, MAX_PROMPT_TEXT)),
    }
}

/// The passages in reading order while they fit `read_page`'s bound; the
/// one that crosses the byte bound keeps the text that fits. True when any
/// was cut.
fn bounded(passages: Vec<Passage>) -> (Vec<Passage>, bool) {
    let mut kept = Vec::with_capacity(passages.len().min(MAX_READ_PASSAGES));
    let mut used = 0;
    for mut passage in passages {
        if kept.len() == MAX_READ_PASSAGES {
            return (kept, true);
        }
        let link = passage.href.as_ref().map_or(0, String::len);
        let room = MAX_READ_BYTES - used;
        if link + passage.text.len() > room {
            if let Some(text_room) = room.checked_sub(link).filter(|&bytes| bytes > 0) {
                let cut = passage.text.floor_char_boundary(text_room);
                passage.text.truncate(cut);
                kept.push(passage);
            }
            return (kept, true);
        }
        used += link + passage.text.len();
        kept.push(passage);
    }
    (kept, false)
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
        // A private tab is never reachable, so it reads as no tab at all.
        if facts.private {
            return Err(Refusal::NotFound);
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
    ///
    /// Deny comes first in `buttons`: rfd makes the first button the
    /// default (Return) one on macOS, both for `NSAlert` and for the
    /// parentless `CFUserNotification` alert, so Return denies. Anything
    /// but a press of an Allow button is `Answer::Deny` (`Answer::from_dialog`).
    ///
    /// `Busy` when this connection already has a prompt on screen or
    /// `MAX_OPEN_PROMPTS` are open; the slot is freed when the person
    /// answers, so a prompt left open by an ended call still counts.
    async fn ask(
        &self,
        connection: u64,
        title: &str,
        body: String,
        buttons: MessageButtons,
        client: &str,
    ) -> Result<Answer, Refusal> {
        let slot = PromptSlot::take(connection)?;
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let app = self.app.clone();
        let client = client.to_owned();
        let dialog = AsyncMessageDialog::new()
            .set_level(MessageLevel::Warning)
            .set_title(title)
            .set_description(body)
            .set_buttons(buttons);
        show_dialog(&self.app, dialog, move |result| {
            drop(slot);
            let answer = Answer::from_dialog(result);
            // `send` fails exactly when the asking call has ended.
            let Err(late) = sender.send(answer) else {
                return;
            };
            if late != Answer::Deny {
                let notice = AsyncMessageDialog::new()
                    .set_level(MessageLevel::Info)
                    .set_title("Zephium")
                    .set_description(format!(
                        "{client}'s request had already ended, so nothing was done. It can ask again."
                    ))
                    .set_buttons(MessageButtons::Ok);
                show_dialog(&app, notice, |_| {});
            }
        });
        // No answer at all (the dialog could not be shown) denies.
        Ok(receiver.await.unwrap_or(Answer::Deny))
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
                title: fence(&title),
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
            let (passages, truncated) = bounded(
                readout
                    .passages
                    .into_iter()
                    .map(|passage| Passage {
                        id: passage.id,
                        role: role_name(passage.role),
                        text: fence(&passage.text),
                        href: passage.href,
                    })
                    .collect(),
            );
            Ok(PageRead {
                read: readout.read,
                title: readout.title.map(|title| fence(&title)),
                passages,
                truncated,
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
        // A private tab is never reachable, so it reads as no tab at all.
        if facts.private {
            return Err(Refusal::NotFound);
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
        let request = Self::step_request(step)?;
        let resolved = page
            .resolve(&step.tab, epoch, &request)
            .map_err(step_failure)?;
        Ok(match (request.verb, step.value.as_deref()) {
            (Verb::Fill, Some(value)) => fill_label(&resolved.label, value),
            _ => resolved.label,
        })
    }

    fn run_step<'a>(&'a self, step: &'a StepIn) -> BoxFuture<'a, Result<(), Refusal>> {
        Box::pin(async move {
            let epoch = self.readable(&step.tab)?;
            let (page, _) = mcp_ports().ok_or(Refusal::Failed)?;
            let request = Self::step_request(step)?;
            // A running Ask reply on this tab pauses, as for any outside input.
            mcp_outside_input(&step.tab);
            page.act(&step.tab, epoch, &request)
                .await
                .map_err(step_failure)
        })
    }

    fn confirm<'a>(
        &'a self,
        connection: u64,
        client: &'a str,
        action: String,
    ) -> BoxFuture<'a, Result<bool, Refusal>> {
        Box::pin(async move {
            let answer = self
                .ask(
                    connection,
                    "Zephium",
                    format!(
                        "{client} wants to {}.",
                        prompt_text(&action, MAX_PROMPT_ACTION)
                    ),
                    // Deny first: it is the default (Return) button.
                    MessageButtons::OkCancelCustom(DENY.into(), ALLOW.into()),
                    client,
                )
                .await?;
            Ok(answer == Answer::Allow)
        })
    }

    fn admit<'a>(
        &'a self,
        connection: u64,
        client: &'a str,
    ) -> BoxFuture<'a, Result<bool, Refusal>> {
        Box::pin(async move {
            let allowed = crate::app_setting("mcp.allowed").unwrap_or_default();
            if zephium_core::preferences::mcp_allowed(&allowed).contains(&client) {
                return Ok(true);
            }
            let answer = self
                .ask(
                    connection,
                    &format!("Allow {client} to use Zephium?"),
                    format!(
                        "{client} wants to see your tabs, read pages and open sites in Zephium. \
                         Closing tabs and page steps will still ask you each time."
                    ),
                    // Deny first: it is the default (Return) button.
                    MessageButtons::YesNoCancelCustom(
                        DENY.into(),
                        ALLOW_ONCE.into(),
                        ALLOW_ALWAYS.into(),
                    ),
                    client,
                )
                .await?;
            Ok(match answer {
                Answer::AllowAlways => {
                    // Read again: another prompt may have added a name meanwhile.
                    let allowed = crate::app_setting("mcp.allowed").unwrap_or_default();
                    self.remember(&allowed, client);
                    true
                }
                Answer::AllowOnce => true,
                Answer::Deny | Answer::Allow => false,
            })
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
