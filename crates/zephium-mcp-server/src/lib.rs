//! rice fork: Zephium as an MCP server for local Claude clients.
//! Protocol only; the browser is reached through `Backend`.
mod handler;
mod socket;
mod tools;

use std::future::Future;
use std::pin::Pin;

pub use socket::McpServer;

/// The deadline every tool call runs under.
pub const CALL_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);

/// How long a client's admission prompt waits for the person. A prompt that
/// times out refuses that call and is asked again on the next one.
pub const ADMIT_DEADLINE: std::time::Duration = std::time::Duration::from_secs(120);

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Clone, Debug)]
pub struct TabInfo {
    pub id: String,
    pub title: String,
    pub url: Option<String>,
    pub space: Option<String>,
    pub active: bool,
}

#[derive(Clone, Debug)]
pub struct Passage {
    pub id: u16,
    pub role: String,
    pub text: String,
    pub href: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PageRead {
    pub read: u64,
    pub title: Option<String>,
    pub passages: Vec<Passage>,
}

#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub last_visit: i64,
}

#[derive(Clone, Debug)]
pub struct StepIn {
    pub tab: String,
    pub read: u64,
    pub verb: String,
    pub target: Option<u16>,
    pub value: Option<String>,
    pub direction: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    NotFound,
    Private,
    Sensitive,
    NotReady,
    PageChanged,
    Denied,
    Invalid(&'static str),
    Failed,
}

impl Refusal {
    pub fn message(self) -> &'static str {
        match self {
            Self::NotFound => "No such tab.",
            Self::Private => "That tab is in a private window.",
            Self::Sensitive => "Zephium does not read this site for assistants.",
            Self::NotReady => "The page has not finished loading. Try again shortly.",
            Self::PageChanged => "The page changed. Read it again, then retry.",
            Self::Denied => "The person declined in Zephium.",
            Self::Invalid(why) => why,
            Self::Failed => "It did not work.",
        }
    }
}

pub trait Backend: Send + Sync + 'static {
    fn list_tabs(&self) -> Vec<TabInfo>;
    fn read_page<'a>(&'a self, tab: &'a str) -> BoxFuture<'a, Result<PageRead, Refusal>>;
    fn search_history(&self, query: &str, limit: u32) -> Result<Vec<HistoryEntry>, Refusal>;
    fn open_tab<'a>(
        &'a self,
        url: &'a str,
        background: bool,
    ) -> BoxFuture<'a, Result<String, Refusal>>;
    fn navigate(&self, tab: &str, url: &str) -> Result<(), Refusal>;
    fn activate(&self, tab: &str) -> Result<(), Refusal>;
    fn close(&self, tab: &str) -> Result<(), Refusal>;
    /// Describes the step for the confirm (`Click "Place order"`), or refuses it.
    fn resolve_step(&self, step: &StepIn) -> Result<String, Refusal>;
    fn run_step<'a>(&'a self, step: &'a StepIn) -> BoxFuture<'a, Result<(), Refusal>>;
    /// Asks the person. True only on an explicit Allow.
    fn confirm<'a>(&'a self, client: &'a str, action: String) -> BoxFuture<'a, bool>;
    /// Admits a client on its first tool call: allowed already, or asks.
    fn admit<'a>(&'a self, client: &'a str) -> BoxFuture<'a, bool>;
    /// The names of the clients connected now, after each connect and disconnect.
    fn connected(&self, clients: &[String]);
    /// One row per tool call. `outcome` is `ok`, `refused`, `timeout`, `error`,
    /// or `cancelled` (the client cancelled the call or the server stopped).
    fn audit(&self, client: &str, tool: &str, tab: Option<&str>, outcome: &str);
}
