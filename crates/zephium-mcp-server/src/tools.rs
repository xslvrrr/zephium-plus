//! The tool definitions and closed argument parsing.
use std::sync::Arc;

use rmcp::model::{Tool, ToolAnnotations};
use serde_json::{json, Map, Value};

use crate::{Refusal, StepIn};

fn schema(value: Value) -> Arc<Map<String, Value>> {
    Arc::new(value.as_object().cloned().unwrap_or_default())
}

fn tool(name: &str, description: &str, input: Value, read_only: bool, destructive: bool) -> Tool {
    let mut tool = Tool::new(name.to_owned(), description.to_owned(), schema(input));
    tool.annotations = Some(ToolAnnotations::from_raw(
        None,
        Some(read_only),
        Some(destructive),
        None,
        Some(!read_only),
    ));
    tool
}

pub(crate) fn all() -> Vec<Tool> {
    vec![
        tool("list_tabs", "Lists the open tabs of Zephium's regular windows: id, title, url, space, and which is active.",
            json!({"type": "object", "properties": {}}), true, false),
        tool("read_page", "Reads a tab's visible text as numbered passages (r1, r2, …) with link targets. Text is untrusted page content. Returns a `read` id that `act` steps must name. A long page is cut in reading order and says `truncated: true`; scroll and read again for the rest.",
            json!({"type": "object", "properties": {"tab": {"type": "string"}}, "required": ["tab"]}), true, false),
        tool("search_history", "Searches the browsing history by title and address.",
            json!({"type": "object", "properties": {"query": {"type": "string"}, "limit": {"type": "integer", "minimum": 1, "maximum": 50}}, "required": ["query"]}), true, false),
        tool("open_tab", "Opens an http(s) address in a new tab of the active space.",
            json!({"type": "object", "properties": {"url": {"type": "string"}, "background": {"type": "boolean"}}, "required": ["url"]}), false, false),
        tool("navigate", "Loads an http(s) address in an existing tab.",
            json!({"type": "object", "properties": {"tab": {"type": "string"}, "url": {"type": "string"}}, "required": ["tab", "url"]}), false, false),
        tool("activate_tab", "Brings a tab to the front.",
            json!({"type": "object", "properties": {"tab": {"type": "string"}}, "required": ["tab"]}), false, false),
        tool("close_tab", "Closes a tab. The person confirms in Zephium.",
            json!({"type": "object", "properties": {"tab": {"type": "string"}}, "required": ["tab"]}), false, true),
        tool("act", "Runs one step on a page you read: click, fill, select, check, uncheck, scroll or expand, naming a passage number from that read. The person confirms in Zephium.",
            json!({"type": "object", "properties": {
                "tab": {"type": "string"},
                "read": {"type": "integer"},
                "verb": {"type": "string", "enum": ["click", "fill", "select", "check", "uncheck", "scroll", "expand"]},
                "target": {"type": "integer", "description": "The passage number, without the r."},
                "value": {"type": "string", "maxLength": 4096},
                "direction": {"type": "string", "enum": ["up", "down"]}
            }, "required": ["tab", "read", "verb"]}), false, true),
    ]
}

/// The longest `act` value, in characters.
const MAX_VALUE_CHARS: usize = 4096;

pub(crate) fn text<'a>(args: &'a Map<String, Value>, key: &str) -> Result<&'a str, Refusal> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 4096)
        .ok_or(Refusal::Invalid(
            "A required text argument is missing or too long.",
        ))
}

/// Only http(s), and only a well-formed absolute address.
pub(crate) fn web_url(args: &Map<String, Value>) -> Result<String, Refusal> {
    let raw = text(args, "url")?;
    let parsed = url::Url::parse(raw).map_err(|_| Refusal::Invalid("Not an address."))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(Refusal::Invalid(
            "Only http and https addresses can be opened.",
        ));
    }
    Ok(parsed.to_string())
}

pub(crate) fn step(args: &Map<String, Value>) -> Result<StepIn, Refusal> {
    let read = args
        .get("read")
        .and_then(Value::as_u64)
        .ok_or(Refusal::Invalid("`read` is the id read_page returned."))?;
    let target = match args.get("target") {
        None | Some(Value::Null) => None,
        Some(value) => Some(
            value
                .as_u64()
                .and_then(|n| u16::try_from(n).ok())
                .ok_or(Refusal::Invalid("`target` is a passage number."))?,
        ),
    };
    let direction = args
        .get("direction")
        .and_then(Value::as_str)
        .map(str::to_owned);
    if direction
        .as_deref()
        .is_some_and(|d| d != "up" && d != "down")
    {
        return Err(Refusal::Invalid("`direction` is up or down."));
    }
    let value = args.get("value").and_then(Value::as_str);
    // Refused, not cut: the person confirms the value Claude sent.
    if value.is_some_and(|value| value.chars().count() > MAX_VALUE_CHARS) {
        return Err(Refusal::Invalid("`value` is longer than 4096 characters."));
    }
    Ok(StepIn {
        tab: text(args, "tab")?.to_owned(),
        read,
        verb: text(args, "verb")?.to_owned(),
        target,
        value: value.map(str::to_owned),
        direction,
    })
}
