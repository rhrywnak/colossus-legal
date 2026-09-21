//! The conversation loop: call → run tools → call again, until the model stops.
//!
//! ```text
//! build body ─► backend.call ─► stop_reason?
//!                   ▲              ├─ tool_use  → run every tool_use block (in parallel),
//!                   │              │              ONE user turn of tool_results, loop
//!                   └──────────────┘
//!                                  ├─ end_turn / stop_sequence → done
//!                                  ├─ refusal / max_tokens     → done, flagged (no tools run)
//!                                  └─ anything else            → named error
//! ```
//!
//! Every message the loop produced (assistant turns AND the tool-result user
//! turns between them) is returned, verbatim, so the caller can append them to its
//! stored history and replay them next time — the append-only rule the provider's
//! thinking signatures depend on.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures::future::join_all;
use serde_json::{json, Value};

use crate::backend::{ChatBackend, ChatTransportError};
use crate::citation::{self, RejectedCitation, VerifiedCitation};
use crate::request::{build_body, ChatRequest, Message, RequestError, Role};
use crate::tools::ChatTool;
use crate::usage::Usage;

// STRUCTURAL: stop_reason vocabulary (wire protocol).
const STOP_TOOL_USE: &str = "tool_use";
const STOP_END_TURN: &str = "end_turn";
const STOP_SEQUENCE: &str = "stop_sequence";
const STOP_REFUSAL: &str = "refusal";
const STOP_MAX_TOKENS: &str = "max_tokens";

/// What happened while a turn ran, streamed to the caller as it happens.
#[derive(Debug, Clone, PartialEq)]
pub enum ChatEvent {
    /// A fragment of the reply's prose.
    TextDelta(String),
    /// The model called a tool.
    ToolStarted {
        /// Tool name.
        name: String,
    },
    /// A tool finished (`ok = false` when it returned an error result).
    ToolFinished {
        /// Tool name.
        name: String,
        /// Whether it succeeded.
        ok: bool,
    },
}

/// How the turn ended, for the caller to store and show.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatOutcome {
    /// Every message produced, in order (assistant, tool-result user, assistant…).
    pub messages: Vec<Message>,
    /// The final `stop_reason`.
    pub stop_reason: String,
    /// Usage summed over every call in the turn.
    pub usage: Usage,
    /// Verified citations of each produced ASSISTANT message, keyed by the index
    /// into `messages`; each paired with the text block it annotates.
    pub citations: HashMap<usize, Vec<(usize, VerifiedCitation)>>,
    /// Citations that failed verification (already dropped from display).
    pub rejected: Vec<RejectedCitation>,
    /// How many model calls the turn took.
    pub rounds: u32,
}

/// Everything a turn can fail with.
#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    /// The request was not buildable.
    #[error(transparent)]
    Request(#[from] RequestError),
    /// The call failed on the wire or in the stream.
    #[error(transparent)]
    Transport(#[from] ChatTransportError),
    /// The model kept calling tools past the configured bound.
    #[error("the model was still calling tools after {max} rounds — the turn is abandoned rather than looping unbounded")]
    ToolRoundsExceeded {
        /// The configured bound.
        max: u32,
    },
    /// A stop reason this loop has no rule for.
    #[error("the model stopped with `{0}`, which this engine has no rule for")]
    UnexpectedStop(String),
}

/// Run one turn of the conversation.
///
/// `events` receives prose and tool progress as it happens; a closed receiver
/// (the browser went away) does not stop the turn — the reply is still stored.
///
/// # Errors
/// See [`ChatError`].
pub async fn run_turn(
    backend: &dyn ChatBackend,
    mut request: ChatRequest,
    tools: &[Arc<dyn ChatTool>],
    max_rounds: u32,
    events: &(dyn Fn(ChatEvent) + Send + Sync),
) -> Result<ChatOutcome, ChatError> {
    // The tools offered are exactly the tools that can run — one source of truth.
    request.tools = tools.iter().map(|t| t.spec()).collect();
    let docs: Vec<String> = request.documents.iter().map(|d| d.text.clone()).collect();
    let doc_refs: Vec<&str> = docs.iter().map(String::as_str).collect();
    let mut outcome = ChatOutcome {
        messages: Vec::new(),
        stop_reason: String::new(),
        usage: Usage::default(),
        citations: HashMap::new(),
        rejected: Vec::new(),
        rounds: 0,
    };
    loop {
        if outcome.rounds >= max_rounds {
            return Err(ChatError::ToolRoundsExceeded { max: max_rounds });
        }
        outcome.rounds += 1;
        let body = build_body(&request)?;
        let on_text = |t: String| events(ChatEvent::TextDelta(t));
        let reply = backend.call(&body, &on_text).await?;
        let content = reply.content.clone();
        record_reply(&mut outcome, &mut request, reply, &doc_refs);

        match outcome.stop_reason.as_str() {
            STOP_TOOL_USE => {
                let user = Message {
                    role: Role::User,
                    content: run_tools(&content, tools, events).await,
                };
                outcome.messages.push(user.clone());
                request.history.push(user);
            }
            STOP_END_TURN | STOP_SEQUENCE | STOP_REFUSAL | STOP_MAX_TOKENS => return Ok(outcome),
            other => return Err(ChatError::UnexpectedStop(other.to_string())),
        }
    }
}

/// Fold one model reply into the outcome and the running history: sum usage,
/// verify its citations (logging every reject), append it verbatim.
fn record_reply(
    outcome: &mut ChatOutcome,
    request: &mut ChatRequest,
    reply: crate::accumulate::AssistantMessage,
    doc_refs: &[&str],
) {
    outcome.usage = outcome.usage.plus(&reply.usage);
    let (kept, rejected) = citation::verify_message(&reply.content, doc_refs);
    for r in &rejected {
        tracing::warn!(document_index = ?r.document_index, reason = %r.reason,
            "chat: a citation failed verification and is not shown");
    }
    outcome.rejected.extend(rejected);
    let assistant = Message {
        role: Role::Assistant,
        content: reply.content,
    };
    outcome.citations.insert(outcome.messages.len(), kept);
    outcome.messages.push(assistant.clone());
    request.history.push(assistant);
    outcome.stop_reason = reply.stop_reason;
}

/// One pending tool call: `(tool_use id, tool name, result)` once awaited.
type ToolCall = Pin<Box<dyn Future<Output = (String, String, Result<String, String>)> + Send>>;

/// Run every `tool_use` block concurrently; return one `tool_result` per call, in
/// the order the calls were made. A tool that fails — or a tool the model named
/// that this turn never offered — returns `is_error: true`: the model is told and
/// can recover, and the person's turn is not thrown away over a bad tool name.
async fn run_tools(
    content: &[Value],
    tools: &[Arc<dyn ChatTool>],
    events: &(dyn Fn(ChatEvent) + Send + Sync),
) -> Vec<Value> {
    // ## Rust Learning: boxing futures of different types into one Vec
    //
    // An offered tool yields the future of its `run`; an unknown one yields an
    // immediately-ready error. Those are two different future TYPES, and a `Vec`
    // holds one type — so each is boxed as `Pin<Box<dyn Future + Send>>`, the
    // common trait-object type, and `join_all` drives them all concurrently.
    let mut calls: Vec<ToolCall> = Vec::new();
    for block in content {
        if block.get("type").and_then(Value::as_str) != Some("tool_use") {
            continue;
        }
        let name = block
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let id = block
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let input = block.get("input").cloned().unwrap_or(Value::Null);
        events(ChatEvent::ToolStarted { name: name.clone() });
        match tools.iter().find(|t| t.name() == name).cloned() {
            Some(tool) => calls.push(Box::pin(async move {
                let result = tool.run(input).await;
                (id, name, result)
            })),
            None => {
                let msg = format!("no tool named `{name}` is available in this conversation");
                calls.push(Box::pin(async move { (id, name, Err(msg)) }));
            }
        }
    }
    let mut out = Vec::with_capacity(calls.len());
    for (id, name, result) in join_all(calls).await {
        let (text, is_error) = match result {
            Ok(text) => (text, false),
            Err(e) => {
                tracing::warn!(tool = %name, error = %e, "chat: a tool call failed; the model is told");
                (e, true)
            }
        };
        events(ChatEvent::ToolFinished {
            name,
            ok: !is_error,
        });
        out.push(json!({"type": "tool_result", "tool_use_id": id,
                        "content": text, "is_error": is_error}));
    }
    out
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
