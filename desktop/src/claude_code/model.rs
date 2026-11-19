//! rice fork: Work's models answered by Claude Code.
//!
//! With `ai.claude-code` on, every model call a Work run makes (the lead,
//! the page reader, the light helper) is one headless `claude -p` call made
//! with the person's Claude Code sign-in. Claude Code is only the model
//! here: its own tools, MCP servers, settings and session files are all
//! off, and the run's tools are described in the system prompt and asked
//! back as structured output, which this client hands to the run as tool
//! calls. Work itself still runs every tool, in its own browser.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;
use zephium_app::work_lead::{LeadModel, WorkLeadModels};
use zephium_core::work::model::{
    WorkModelClient, WorkModelEntry, WorkModelError, WorkModelEvent, WorkModelFuture,
    WorkModelMessage, WorkModelOutcome, WorkModelPart, WorkModelProvider, WorkModelRef,
    WorkModelRequest, WorkModelRole, WorkModelStop, WorkModelSupports, WorkModelToolCall,
    WorkModelToolChoice, WorkModelUsage, WorkModelWire,
};

static NEXT_CALL: AtomicU64 = AtomicU64::new(1);

/// What every answer must be: words for the person, and the tools to call.
const SCHEMA: &str = r#"{"type":"object","properties":{"text":{"type":"string"},"tool_calls":{"type":"array","items":{"type":"object","properties":{"name":{"type":"string"},"input":{"type":"object"}},"required":["name","input"]}}},"required":["text","tool_calls"]}"#;

const PROTOCOL: &str = "\n\n# How to answer\n\
You cannot run tools yourself. Every reply is one JSON object: `text` is what you say \
(it may be empty), and `tool_calls` lists the tools to call now, in order, each with the \
tool's `name` and an `input` object matching that tool's input schema. The results come \
back in the next message, marked with the call's id. Leave `tool_calls` empty only when \
you are done and have nothing more to call.";

/// Whether Claude Code answers Work's model calls: it is on and installed.
/// It signs in with its own subscription, so no key is needed.
pub(crate) fn answers_work() -> bool {
    crate::app_setting("ai.claude-code").as_deref() == Some("true") && super::locate().is_some()
}

/// The models a Work run uses when Claude Code answers: `None` while it is
/// off or not installed, and the run falls back to the person's choices.
pub(crate) fn lead_models() -> Option<WorkLeadModels> {
    if !answers_work() {
        return None;
    }
    let model = |role, alias: Option<&'static str>, name: &str| LeadModel {
        entry: entry(role, alias, name),
        client: Arc::new(ClaudeCodeModel { alias }),
    };
    Some(WorkLeadModels {
        // The person's own Claude Code default leads; the helpers are lighter.
        lead: model(WorkModelRole::Lead, None, "Claude Code"),
        page: model(WorkModelRole::Page, Some("sonnet"), "Claude Code · Sonnet"),
        light: model(WorkModelRole::Light, Some("haiku"), "Claude Code · Haiku"),
    })
}

fn entry(role: WorkModelRole, alias: Option<&str>, name: &str) -> WorkModelEntry {
    let model = alias.unwrap_or("default");
    WorkModelEntry {
        id: format!("claude-code/{model}"),
        model: WorkModelRef {
            provider: WorkModelProvider::Anthropic,
            wire: WorkModelWire::AnthropicMessages,
            model: model.to_owned(),
        },
        display_name: name.to_owned(),
        roles: vec![role],
        recommended: false,
        context_window: 200_000,
        max_output: 32_000,
        supports: WorkModelSupports {
            tools: true,
            vision: false,
            prompt_cache: false,
            reasoning: false,
            native_search: false,
        },
        price: None,
    }
}

struct ClaudeCodeModel {
    /// `--model`; `None` keeps the person's Claude Code default.
    alias: Option<&'static str>,
}

impl WorkModelClient for ClaudeCodeModel {
    fn call<'a>(
        &'a self,
        request: WorkModelRequest,
        events: &'a (dyn Fn(WorkModelEvent) + Send + Sync),
    ) -> WorkModelFuture<'a> {
        Box::pin(async move {
            let claude = super::locate().ok_or(WorkModelError::MissingKey)?;
            let dir = std::env::temp_dir().join("zephium-claude-code-work");
            tokio::fs::create_dir_all(&dir)
                .await
                .map_err(|_| WorkModelError::Network)?;
            let mut command = tokio::process::Command::new(claude);
            command
                .args(["-p", "--output-format", "json", "--tools", ""])
                .args(["--strict-mcp-config", "--setting-sources", "project"])
                .arg("--no-session-persistence")
                .arg("--system-prompt")
                .arg(system(&request))
                .arg("--json-schema")
                .arg(SCHEMA);
            if let Some(alias) = self.alias {
                command.arg("--model").arg(alias);
            }
            command
                .current_dir(&dir)
                .env("PATH", super::search_path())
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                // Dropping the call (the run stopped) stops Claude Code too.
                .kill_on_drop(true);
            let mut child = command.spawn().map_err(|_| WorkModelError::Network)?;
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(transcript(&request.messages).as_bytes())
                    .await
                    .map_err(|_| WorkModelError::Protocol)?;
            }
            let output = child
                .wait_with_output()
                .await
                .map_err(|_| WorkModelError::Network)?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let Some(result) = stdout
                .lines()
                .rev()
                .find_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
            else {
                return Err(if output.status.success() {
                    WorkModelError::Protocol
                } else {
                    WorkModelError::Network
                });
            };
            if result.get("is_error").and_then(Value::as_bool) == Some(true) {
                return Err(failure(&result));
            }
            let answer = result
                .get("structured_output")
                .filter(|value| value.is_object())
                .cloned()
                .or_else(|| {
                    result
                        .get("result")
                        .and_then(Value::as_str)
                        .and_then(|text| serde_json::from_str::<Value>(text).ok())
                })
                .ok_or(WorkModelError::Protocol)?;
            Ok(outcome(&request, &answer, &result, events))
        })
    }
}

/// The run's own system text, its tools, and how to answer with them.
fn system(request: &WorkModelRequest) -> String {
    let mut text = request
        .system
        .iter()
        .map(|block| block.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    if !request.tools.is_empty() {
        text.push_str("\n\n# Tools\n");
        for tool in &request.tools {
            text.push_str(&format!(
                "\n## {}\n{}\nInput schema: {}\n",
                tool.name, tool.description, tool.schema
            ));
        }
    }
    text.push_str(PROTOCOL);
    match &request.tool_choice {
        WorkModelToolChoice::Auto => {}
        WorkModelToolChoice::Required => {
            text.push_str("\nThis reply must call at least one tool.");
        }
        WorkModelToolChoice::Named(name) => {
            text.push_str(&format!("\nThis reply must call the tool `{name}`."));
        }
    }
    text
}

/// The conversation so far, as tagged turns.
fn transcript(messages: &[WorkModelMessage]) -> String {
    let mut text = String::new();
    for message in messages {
        match message {
            WorkModelMessage::User(parts) => {
                text.push_str("<user>\n");
                parts_text(&mut text, parts);
                text.push_str("</user>\n\n");
            }
            WorkModelMessage::Assistant(parts) => {
                text.push_str("<assistant>\n");
                parts_text(&mut text, parts);
                text.push_str("</assistant>\n\n");
            }
            WorkModelMessage::ToolResults(results) => {
                text.push_str("<tool_results>\n");
                for result in results {
                    text.push_str(&format!(
                        "<result id=\"{}\" error=\"{}\">\n{}\n</result>\n",
                        result.call, result.is_error, result.content
                    ));
                }
                text.push_str("</tool_results>\n\n");
            }
        }
    }
    text.push_str("Give your next reply.");
    text
}

fn parts_text(text: &mut String, parts: &[WorkModelPart]) {
    for part in parts {
        match part {
            WorkModelPart::Text(words) => {
                text.push_str(words);
                text.push('\n');
            }
            WorkModelPart::Image { .. } => text.push_str("[an image, not shown]\n"),
            WorkModelPart::ToolCall(call) => text.push_str(&format!(
                "<tool_call id=\"{}\" name=\"{}\">{}</tool_call>\n",
                call.id, call.name, call.arguments
            )),
            WorkModelPart::Replay(_) => {}
        }
    }
}

fn outcome(
    request: &WorkModelRequest,
    answer: &Value,
    result: &Value,
    events: &(dyn Fn(WorkModelEvent) + Send + Sync),
) -> WorkModelOutcome {
    let mut assistant = Vec::new();
    let words = answer
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if !words.is_empty() {
        events(WorkModelEvent::Text(words.to_owned()));
        assistant.push(WorkModelPart::Text(words.to_owned()));
    }
    let calls = answer
        .get("tool_calls")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for call in calls {
        let Some(name) = call.get("name").and_then(Value::as_str) else {
            continue;
        };
        // Only tools the run offered are handed back as calls.
        if !request.tools.iter().any(|tool| tool.name == name) {
            continue;
        }
        let arguments = call
            .get("input")
            .filter(|input| input.is_object())
            .cloned()
            .unwrap_or_else(|| json!({}));
        let call = WorkModelToolCall {
            id: format!("cc_{}", NEXT_CALL.fetch_add(1, Ordering::Relaxed)),
            name: name.to_owned(),
            arguments,
        };
        events(WorkModelEvent::ToolCall(call.clone()));
        assistant.push(WorkModelPart::ToolCall(call));
    }
    let called = assistant
        .iter()
        .any(|part| matches!(part, WorkModelPart::ToolCall(_)));
    let tokens = |key: &str| {
        result
            .pointer(&format!("/usage/{key}"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    let cached = tokens("cache_read_input_tokens");
    let written = tokens("cache_creation_input_tokens");
    WorkModelOutcome {
        stop: if called {
            WorkModelStop::ToolUse
        } else {
            WorkModelStop::EndTurn
        },
        usage: WorkModelUsage {
            input_tokens: tokens("input_tokens") + cached + written,
            cached_input_tokens: cached,
            cache_write_input_tokens: written,
            output_tokens: tokens("output_tokens"),
            reasoning_tokens: 0,
            // Billed to the person's Claude Code plan, not to the run.
            cost_micros: Some(0),
        },
        assistant,
    }
}

/// A failed `claude -p`, as the closed reasons a run understands.
fn failure(result: &Value) -> WorkModelError {
    let status = result.get("api_error_status").and_then(Value::as_u64);
    let text = result
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    match status {
        Some(401 | 403) => WorkModelError::Unauthorized,
        Some(429) => WorkModelError::RateLimited {
            retry_after_ms: None,
        },
        Some(500..=599) => WorkModelError::Overloaded,
        _ if text.contains("prompt is too long") => WorkModelError::ContextTooLong,
        _ if text.contains("log in") || text.contains("login") => WorkModelError::Unauthorized,
        _ if text.contains("limit") => WorkModelError::RateLimited {
            retry_after_ms: None,
        },
        _ => WorkModelError::Protocol,
    }
}
