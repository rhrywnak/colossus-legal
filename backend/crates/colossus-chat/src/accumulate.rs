//! Full-fidelity reassembly of one streamed assistant message.
//!
//! The extraction engine's accumulator keeps only joined text, because that is
//! all extraction consumes. A CONVERSATION needs every block back exactly as the
//! provider sent it, because the next turn replays the history:
//!
//! - `thinking` blocks carry a `signature` the API checks on replay — edit or drop
//!   one mid-history and later thinking is invalidated.
//! - `tool_use` blocks carry the `id` their `tool_result` must answer.
//! - `compaction` blocks REPLACE everything before them on the next request; lose
//!   one and the provider re-reads (and re-bills) the whole uncompacted history.
//! - `text` blocks carry `citations`, which is the whole point of this engine.
//!
//! So each block is rebuilt as a verbatim `serde_json::Value`, and anything this
//! module does not recognise is a NAMED ERROR rather than being skipped. The
//! extraction decoder's silent `#[serde(other)]` arm is exactly the hole that would
//! let a citation or a signature vanish without a trace (Standing Rule 1).

use serde_json::{Map, Value};

use crate::blocks::{
    append_str, grow, grow_strings, index_field, malformed, push_array, set_field, str_field,
};
use crate::sse::SseError;
use crate::transport::{EventFold, Progress};
use crate::usage::Usage;

// STRUCTURAL: Anthropic Messages API streaming vocabulary — wire protocol, not settings.
const EV_MESSAGE_START: &str = "message_start";
const EV_BLOCK_START: &str = "content_block_start";
const EV_BLOCK_DELTA: &str = "content_block_delta";
const EV_BLOCK_STOP: &str = "content_block_stop";
const EV_MESSAGE_DELTA: &str = "message_delta";
const EV_MESSAGE_STOP: &str = "message_stop";
const EV_ERROR: &str = "error";
const BLOCK_TEXT: &str = "text";
const BLOCK_THINKING: &str = "thinking";
const BLOCK_REDACTED_THINKING: &str = "redacted_thinking";
const BLOCK_TOOL_USE: &str = "tool_use";
const BLOCK_COMPACTION: &str = "compaction";
/// The block types this engine can replay faithfully. Anything else is refused.
const KNOWN_BLOCKS: [&str; 5] = [
    BLOCK_TEXT,
    BLOCK_THINKING,
    BLOCK_REDACTED_THINKING,
    BLOCK_TOOL_USE,
    BLOCK_COMPACTION,
];

/// Everything that can go wrong rebuilding a chat message from its stream.
#[derive(Debug, thiserror::Error)]
pub enum ChatStreamError {
    /// Framing failed underneath us.
    #[error(transparent)]
    Sse(#[from] SseError),
    /// A payload was not JSON, or lacked a field the protocol guarantees.
    #[error("a server-sent event was malformed ({reason}); payload was: {preview}")]
    Malformed {
        /// What was wrong.
        reason: String,
        /// The first characters of the payload.
        preview: String,
    },
    /// A content block type this engine cannot replay faithfully.
    #[error(
        "the provider opened a `{kind}` content block, which this engine does not know how \
         to store and replay — refusing rather than dropping it from the history"
    )]
    UnknownBlock {
        /// The block's `type`.
        kind: String,
    },
    /// A delta type this engine does not recognise.
    #[error(
        "the provider sent a `{kind}` delta for block {index}, which this engine does not \
         recognise — refusing rather than silently dropping part of the answer"
    )]
    UnknownDelta {
        /// The delta's `type`.
        kind: String,
        /// Which block it was for.
        index: usize,
    },
    /// A `tool_use` block's streamed input was not valid JSON.
    #[error("the input streamed for tool call `{name}` is not valid JSON ({source})")]
    ToolInput {
        /// The tool's name.
        name: String,
        /// The parse failure.
        #[source]
        source: serde_json::Error,
    },
    /// The provider sent an `error` event mid-stream.
    #[error("provider sent an error event mid-stream: {kind}: {message}")]
    ProviderEvent {
        /// Anthropic's `error.type`.
        kind: String,
        /// Anthropic's `error.message`.
        message: String,
    },
    /// The stream ended without `message_stop`.
    #[error("the event stream ended after {events_seen} events without `message_stop` — the reply is INCOMPLETE and is not stored as an answer")]
    Incomplete {
        /// How many events arrived first.
        events_seen: usize,
    },
    /// The stream completed but never said why it stopped.
    #[error("the event stream reached `message_stop` without a `stop_reason` — cannot tell a finished reply from a cut-off one")]
    MissingStopReason,
}

/// One finished assistant message, block for block.
#[derive(Debug, Clone, PartialEq)]
pub struct AssistantMessage {
    /// Every content block, verbatim, in index order — the exact array to replay.
    pub content: Vec<Value>,
    /// Why the model stopped (`end_turn`, `tool_use`, `max_tokens`, `refusal`, …).
    pub stop_reason: String,
    /// Token accounting, including cache reads/writes.
    pub usage: Usage,
}

impl AssistantMessage {
    /// The prose of the reply: every `text` block joined in order, no separator
    /// (citation-split blocks are fragments of one paragraph).
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter(|b| b.get("type").and_then(Value::as_str) == Some(BLOCK_TEXT))
            .filter_map(|b| b.get("text").and_then(Value::as_str))
            .collect()
    }
}

/// Folds decoded events into an [`AssistantMessage`].
///
/// Also buffers the prose deltas as they arrive so a caller can forward them to a
/// browser while the message is still streaming ([`ChatAccumulator::take_text`]).
#[derive(Debug, Default)]
pub struct ChatAccumulator {
    /// Blocks under construction, by the index the provider assigns.
    blocks: Vec<Option<Value>>,
    /// Streamed tool input fragments, by block index.
    tool_json: Vec<String>,
    usage: Usage,
    stop_reason: Option<String>,
    saw_stop: bool,
    events_seen: usize,
    /// Prose that arrived since the caller last took it.
    fresh_text: String,
    /// How much of a malformed payload an error quotes (the caller's configuration).
    preview_chars: usize,
}

impl ChatAccumulator {
    /// Create an empty accumulator. `preview_chars` bounds how much of a malformed
    /// payload an error quotes — the caller's configuration, never a constant here.
    pub fn new(preview_chars: usize) -> Self {
        Self {
            preview_chars,
            ..Self::default()
        }
    }

    /// Take the prose that arrived since the last call (empty when none did).
    pub fn take_text(&mut self) -> String {
        std::mem::take(&mut self.fresh_text)
    }

    fn fold(&mut self, event: &Value, payload: &str) -> Result<Progress, ChatStreamError> {
        let p = self.preview_chars;
        match str_field(event, "type", payload, p)? {
            EV_MESSAGE_START => {
                if let Some(usage) = event.get("message").and_then(|m| m.get("usage")) {
                    self.usage.absorb(usage);
                }
            }
            EV_BLOCK_START => self.open(event, payload)?,
            EV_BLOCK_DELTA => self.delta(event, payload)?,
            EV_BLOCK_STOP => self.close(event, payload)?,
            EV_MESSAGE_DELTA => {
                if let Some(reason) = event
                    .get("delta")
                    .and_then(|d| d.get("stop_reason"))
                    .and_then(Value::as_str)
                {
                    self.stop_reason = Some(reason.to_string());
                }
                if let Some(usage) = event.get("usage") {
                    self.usage.absorb(usage);
                }
            }
            EV_MESSAGE_STOP => {
                self.saw_stop = true;
                return Ok(Progress::Done);
            }
            EV_ERROR => {
                let err = event.get("error");
                let get = |k: &str| {
                    err.and_then(|e| e.get(k))
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string()
                };
                return Err(ChatStreamError::ProviderEvent {
                    kind: get("type"),
                    message: get("message"),
                });
            }
            // `ping`, and any event type added later, carries nothing we store.
            // Events (unlike blocks and deltas) are envelope, not content, so
            // ignoring a new one cannot lose part of the answer.
            _ => {}
        }
        Ok(Progress::Continue)
    }

    fn open(&mut self, event: &Value, payload: &str) -> Result<(), ChatStreamError> {
        let p = self.preview_chars;
        let index = index_field(event, payload, p)?;
        let block = event
            .get("content_block")
            .cloned()
            .ok_or_else(|| malformed("content_block_start without content_block", payload, p))?;
        let kind = str_field(&block, "type", payload, p)?.to_string();
        if !KNOWN_BLOCKS.contains(&kind.as_str()) {
            return Err(ChatStreamError::UnknownBlock { kind });
        }
        grow(&mut self.blocks, index);
        grow_strings(&mut self.tool_json, index);
        self.blocks[index] = Some(block);
        Ok(())
    }

    fn delta(&mut self, event: &Value, payload: &str) -> Result<(), ChatStreamError> {
        let p = self.preview_chars;
        let index = index_field(event, payload, p)?;
        let delta = event
            .get("delta")
            .ok_or_else(|| malformed("content_block_delta without delta", payload, p))?;
        let kind = str_field(delta, "type", payload, p)?;
        let block = self
            .blocks
            .get_mut(index)
            .and_then(Option::as_mut)
            .ok_or_else(|| malformed("delta for a block that was never opened", payload, p))?;
        match kind {
            "text_delta" => {
                let text = str_field(delta, "text", payload, p)?;
                append_str(block, "text", text);
                self.fresh_text.push_str(text);
            }
            "citations_delta" => {
                let citation = delta
                    .get("citation")
                    .cloned()
                    .ok_or_else(|| malformed("citations_delta without citation", payload, p))?;
                push_array(block, "citations", citation);
            }
            "thinking_delta" => {
                append_str(block, "thinking", str_field(delta, "thinking", payload, p)?)
            }
            "signature_delta" => append_str(
                block,
                "signature",
                str_field(delta, "signature", payload, p)?,
            ),
            "input_json_delta" => {
                let part = str_field(delta, "partial_json", payload, p)?;
                if let Some(buf) = self.tool_json.get_mut(index) {
                    buf.push_str(part);
                }
            }
            "compaction_delta" => {
                // `content` may legitimately be null on a failed compaction; the
                // block then carries null, exactly as the provider said.
                let content = delta.get("content").cloned().unwrap_or(Value::Null);
                set_field(block, "content", content);
            }
            other => {
                return Err(ChatStreamError::UnknownDelta {
                    kind: other.to_string(),
                    index,
                })
            }
        }
        Ok(())
    }

    fn close(&mut self, event: &Value, payload: &str) -> Result<(), ChatStreamError> {
        let p = self.preview_chars;
        let index = index_field(event, payload, p)?;
        let json = self.tool_json.get(index).cloned().unwrap_or_default();
        let Some(Some(block)) = self.blocks.get_mut(index) else {
            return Err(malformed(
                "content_block_stop for a block never opened",
                payload,
                p,
            ));
        };
        if block.get("type").and_then(Value::as_str) == Some(BLOCK_TOOL_USE) {
            let name = block
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            // An empty stream of input is a tool called with no arguments: `{}`.
            let input = if json.trim().is_empty() {
                Value::Object(Map::new())
            } else {
                serde_json::from_str(&json)
                    .map_err(|source| ChatStreamError::ToolInput { name, source })?
            };
            set_field(block, "input", input);
        }
        Ok(())
    }
}

impl EventFold for ChatAccumulator {
    type Output = AssistantMessage;
    type Error = ChatStreamError;

    fn push(&mut self, payload: &str) -> Result<Progress, ChatStreamError> {
        let p = self.preview_chars;
        let event: Value = serde_json::from_str(payload)
            .map_err(|e| malformed(&format!("not JSON: {e}"), payload, p))?;
        self.events_seen += 1;
        self.fold(&event, payload)
    }

    fn events_seen(&self) -> usize {
        self.events_seen
    }

    fn finish(self) -> Result<AssistantMessage, ChatStreamError> {
        if !self.saw_stop {
            return Err(ChatStreamError::Incomplete {
                events_seen: self.events_seen,
            });
        }
        let stop_reason = self.stop_reason.ok_or(ChatStreamError::MissingStopReason)?;
        Ok(AssistantMessage {
            // `flatten` drops the `None` slots of any index the provider skipped.
            content: self.blocks.into_iter().flatten().collect(),
            stop_reason,
            usage: self.usage,
        })
    }
}

#[cfg(test)]
#[path = "accumulate_tests.rs"]
mod tests;
