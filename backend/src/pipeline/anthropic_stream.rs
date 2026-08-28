//! Server-sent-event decoding and message accumulation for the Anthropic
//! Messages API.
//!
//! ## The incident this module exists to fix (2026-08-28)
//!
//! A 36-page `court_transcript` run (Opus 5, `max_tokens = 64000`) failed at
//! exactly `600.0s` in `step_llm_extract_pass1` with
//! `HttpError: Http client error: error sending request`. Nothing was wrong with
//! the prompt, the model, or the network: the call was a single
//! POST-and-wait against `/v1/messages`, and the whole-request timeout on the
//! HTTP client expired while the model was still generating. Anthropic's own
//! documentation states that a non-streaming Messages request must complete
//! inside roughly ten minutes, and that a large `max_tokens` without streaming
//! risks the connection being dropped while idle. The accepted remedy is the
//! streaming Messages API.
//!
//! Worse than the failure was what followed it: the error classified as
//! *retryable*, so Restate re-ran the step and spent the same money reaching the
//! same wall. See `crate::llm_retry` and
//! `crate::pipeline::workflow_steps::llm_extract` for the retry half of the fix.
//!
//! ## What lives here, and what deliberately does not
//!
//! This module is **pure**. It converts bytes into events and events into a
//! finished message, and it performs no I/O whatsoever. That is what lets the
//! whole protocol — including the truncation case that must stay TERMINAL — be
//! asserted from a string literal in a unit test, with no Anthropic call and no
//! money spent. The socket half lives in
//! [`crate::pipeline::anthropic_transport`].
//!
//! ## The shape being rebuilt
//!
//! A streamed response arrives as a sequence of events:
//!
//! ```text
//! message_start        → the message envelope: id, and input token usage
//! content_block_start  → one per content block, carrying its type
//! content_block_delta  → many; `text_delta` carries the prose
//! content_block_stop   → one per block
//! message_delta        → THE stop_reason, plus final output token usage
//! message_stop         → end of message
//! ```
//!
//! The accumulator rebuilds exactly the three things the pipeline consumed from
//! the old single-shot response body — joined text, `stop_reason`, and `usage` —
//! so nothing downstream (parsing, grounding, cost accounting) changes.
//!
//! ## Domain note: why `stop_reason` is required rather than optional here
//!
//! `stop_reason` arrives in `message_delta` and nowhere else. It is the single
//! field that says whether the answer above it is the whole answer — census R-3
//! (see [`crate::pipeline::truncation`]) exists because it used to be discarded,
//! and `repair_json` closed the truncated array so convincingly that a cut-off
//! extraction was stored as a complete one. A streamed message that reaches
//! `message_stop` without ever carrying a `stop_reason` therefore fails loudly
//! ([`StreamError::MissingStopReason`]) instead of returning `None`. Returning
//! `None` would be indistinguishable from a provider that does not report the
//! field at all, and would silently disarm the truncation gate for every call.

use serde::Deserialize;

/// The SSE field prefix carrying an event's JSON payload.
///
/// CONST: the Server-Sent Events wire format (W3C `text/event-stream`), not a
/// setting. A deployment cannot choose a different spelling for it.
const SSE_DATA_FIELD: &str = "data:";

/// How much of an unparseable payload is quoted back in an error message.
///
/// CONST: an error-message ergonomics value, not a tunable. Long enough to
/// identify which event went wrong, short enough that a malformed multi-megabyte
/// payload cannot flood `pipeline_jobs.error`.
const MALFORMED_PREVIEW_CHARS: usize = 200;

// ─────────────────────────────────────────────────────────────────
// The finished message
// ─────────────────────────────────────────────────────────────────

/// A streamed Anthropic message, reassembled.
///
/// Deliberately the same three pieces of information the non-streaming response
/// body carried, in the same shapes, so the adapter above can build an identical
/// `LlmCallResult` and nothing downstream can tell the transports apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamedMessage {
    /// Every `text` content block, joined in block-index order with `\n`.
    ///
    /// Matches the join the non-streaming path performed over the response's
    /// content blocks — `tool_use`, `thinking`, and image blocks are skipped
    /// there and skipped here.
    pub text: String,

    /// Input tokens, from `message_start.message.usage`.
    ///
    /// `None` means the provider did not report them — distinguishable from
    /// `Some(0)` (Standing Rule 1).
    pub input_tokens: Option<u64>,

    /// Output tokens, from `message_delta.usage` (the cumulative final count),
    /// falling back to `message_start` when no delta reported one.
    pub output_tokens: Option<u64>,

    /// Why the model stopped, from `message_delta.delta.stop_reason`.
    ///
    /// Always populated on a well-formed stream — see the module doc for why
    /// its absence is an error rather than a `None`.
    pub stop_reason: String,

    /// The provider's message id (`msg_…`), from `message_start.message.id`.
    ///
    /// `None` only if the stream somehow completed without a `message_start`,
    /// which [`MessageAccumulator::finish`] does not otherwise police — the id
    /// is for trace correlation, not for correctness.
    pub message_id: Option<String>,
}

/// Everything that can go wrong turning a byte stream into a message.
///
/// ## Rust Learning: `thiserror` and the `#[error(...)]` attribute
///
/// `thiserror::Error` derives `std::error::Error` and writes the `Display` impl
/// from the format string on each variant. The braces interpolate the variant's
/// own fields by name, so the message and the data it describes cannot drift
/// apart — there is no second copy of the wording to keep in sync.
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    /// The provider sent an `error` event mid-stream (`overloaded_error`,
    /// `api_error`, …). Per the 2026-08-28 ruling this gets no special
    /// handling: it fails the call and the retry policy decides what happens
    /// next — which, at the shipped `LLM_RETRY_MAX=0`, is "a human clicks
    /// Re-process".
    #[error("provider sent an error event mid-stream: {kind}: {message}")]
    ProviderEvent {
        /// Anthropic's `error.type` discriminator, e.g. `overloaded_error`.
        kind: String,
        /// Anthropic's human-readable `error.message`.
        message: String,
    },

    /// The stream ended without a `message_stop`. The bytes we did receive are
    /// a PARTIAL answer and are discarded rather than parsed — the same
    /// reasoning as the truncation gate, for the same reason (`repair_json`
    /// would close the partial JSON and it would store as a complete result).
    #[error(
        "the event stream ended after {events_seen} events without a `message_stop` — \
         the response is INCOMPLETE and is discarded rather than parsed, because a \
         partial response repairs into plausible JSON and would otherwise be stored \
         as a complete result"
    )]
    Incomplete {
        /// How many events were decoded before the stream ended, so an operator
        /// can tell "the connection died immediately" from "it died at the end".
        events_seen: usize,
    },

    /// The stream completed but never carried a `stop_reason`.
    ///
    /// See the module doc: reporting `None` here would disarm the truncation
    /// gate for every call, which is the exact defect census R-3 closed.
    #[error(
        "the event stream reached `message_stop` without ever carrying a `stop_reason` \
         — the truncation gate cannot tell a complete answer from one cut off at the \
         max_tokens ceiling, so the response is discarded rather than trusted"
    )]
    MissingStopReason,

    /// A `data:` payload was not the JSON this module knows how to read.
    #[error("could not parse a server-sent event as JSON ({source}); payload was: {preview}")]
    MalformedEvent {
        /// The first [`MALFORMED_PREVIEW_CHARS`] characters of the payload.
        preview: String,
        /// The underlying serde failure.
        #[source]
        source: serde_json::Error,
    },

    /// A complete SSE line was not valid UTF-8. Cannot happen against
    /// Anthropic, which sends JSON; surfaced rather than lossily replaced so a
    /// mis-framed or corrupted stream is not silently turned into mojibake that
    /// then fails much further downstream as a confusing parse error.
    #[error("a server-sent event line was not valid UTF-8: {source}")]
    InvalidUtf8 {
        /// The underlying decode failure.
        #[source]
        source: std::str::Utf8Error,
    },
}

// ─────────────────────────────────────────────────────────────────
// Wire types
// ─────────────────────────────────────────────────────────────────

/// One decoded SSE payload.
///
/// ## Rust Learning: `#[serde(tag = "…")]` internally-tagged enums
///
/// Anthropic's events are JSON objects that carry their own discriminator in a
/// `type` field rather than being wrapped in one. `tag = "type"` tells serde to
/// read that field to pick the variant and then deserialize the REMAINING fields
/// into it. `rename_all = "snake_case"` maps `MessageStart` to `"message_start"`
/// so the Rust names stay idiomatic while matching the wire exactly.
///
/// `#[serde(other)]` on a unit variant is the forward-compatibility hatch: an
/// event type Anthropic adds later deserializes to [`WireEvent::Unknown`] and is
/// ignored, instead of failing every extraction the day they ship it. It is only
/// legal on a unit variant, which is why `Unknown` carries nothing.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WireEvent {
    MessageStart {
        message: WireMessageStart,
    },
    ContentBlockStart {
        index: usize,
        content_block: WireContentBlock,
    },
    ContentBlockDelta {
        index: usize,
        delta: WireContentDelta,
    },
    ContentBlockStop {},
    MessageDelta {
        delta: WireMessageDelta,
        usage: WireDeltaUsage,
    },
    MessageStop,
    Error {
        error: WireErrorBody,
    },
    #[serde(other)]
    Unknown,
}

/// The `message` envelope on `message_start`.
#[derive(Debug, Deserialize)]
struct WireMessageStart {
    id: String,
    usage: WireStartUsage,
}

/// `message_start.message.usage`. Anthropic reports the input count here and
/// nowhere else; the output count present at this point is a placeholder that
/// `message_delta` supersedes.
#[derive(Debug, Deserialize)]
struct WireStartUsage {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
}

/// A content block's header. Only its `type` matters to us.
#[derive(Debug, Deserialize)]
struct WireContentBlock {
    #[serde(rename = "type")]
    kind: String,
}

/// A `content_block_delta.delta`. Only `text_delta` carries prose; `thinking`
/// and tool-input deltas are decoded so they can be explicitly ignored rather
/// than tripping the unknown-payload error.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WireContentDelta {
    TextDelta {
        text: String,
    },
    #[serde(other)]
    Other,
}

/// `message_delta.delta` — the home of `stop_reason`.
#[derive(Debug, Deserialize)]
struct WireMessageDelta {
    #[serde(default)]
    stop_reason: Option<String>,
}

/// `message_delta.usage` — the final, cumulative output token count.
#[derive(Debug, Deserialize)]
struct WireDeltaUsage {
    #[serde(default)]
    output_tokens: Option<u64>,
}

/// The body of an `error` event.
#[derive(Debug, Deserialize)]
struct WireErrorBody {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    message: String,
}

/// The content-block type whose deltas carry prose.
///
/// CONST: Anthropic Messages API wire vocabulary, like
/// [`crate::pipeline::truncation::STOP_REASON_MAX_TOKENS`] — not a setting.
const BLOCK_TYPE_TEXT: &str = "text";

// ─────────────────────────────────────────────────────────────────
// Byte framing
// ─────────────────────────────────────────────────────────────────

/// Incremental `text/event-stream` framer.
///
/// Bytes arrive in arbitrary chunks that split lines and even split multi-byte
/// characters. The decoder buffers **bytes** and only decodes a line once its
/// terminating `\n` has arrived, so a UTF-8 sequence straddling a chunk boundary
/// is reassembled before anyone tries to read it.
///
/// ## Rust Learning: why the buffer is `Vec<u8>` and not `String`
///
/// `String` must be valid UTF-8 at all times, so pushing a half-decoded
/// character into one is not merely unwise — it will not compile without a
/// lossy conversion, and lossy is exactly what we must not do here (a `U+FFFD`
/// substituted into a document quote is a grounding failure much later, in a
/// place that gives no hint where it came from). Buffering raw bytes and
/// decoding whole lines keeps the failure at the boundary where it happened.
#[derive(Debug, Default)]
pub struct SseDecoder {
    /// Bytes received but not yet terminated by a newline.
    pending: Vec<u8>,
    /// `data:` field values collected for the event currently being framed.
    /// SSE allows several `data:` lines per event; they join with `\n`.
    data: String,
}

impl SseDecoder {
    /// Create an empty decoder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed the next chunk of socket bytes; return every COMPLETE event payload
    /// it completed, in order.
    ///
    /// An empty return is normal and meaningful: it means the chunk carried only
    /// part of a line, or only SSE fields we ignore (`event:`, `id:`, comments).
    /// It is NOT a failure and, importantly, it is NOT evidence of an idle
    /// stream — see [`crate::pipeline::anthropic_transport`] for why the idle
    /// clock is reset on decoded EVENTS rather than on received bytes.
    ///
    /// # Errors
    ///
    /// [`StreamError::InvalidUtf8`] if a complete line is not valid UTF-8.
    pub fn push_bytes(&mut self, chunk: &[u8]) -> Result<Vec<String>, StreamError> {
        self.pending.extend_from_slice(chunk);
        let mut payloads = Vec::new();

        // ## Rust Learning: draining a buffer by index rather than by iterator
        //
        // We cannot iterate `self.pending` while also mutating it, so the loop
        // finds the next newline by position, splits the buffer with
        // `Vec::drain`, and repeats. `drain(..=idx)` removes the line AND its
        // terminator in one move and yields them; collecting into a `Vec<u8>`
        // ends the borrow before the next iteration begins.
        while let Some(idx) = self.pending.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=idx).collect();
            let line =
                std::str::from_utf8(&line).map_err(|source| StreamError::InvalidUtf8 { source })?;
            // Trim the terminator in both framings — a `\r\n` stream is legal
            // SSE and a stray `\r` left on a JSON payload would fail the parse.
            let line = line.trim_end_matches('\n').trim_end_matches('\r');

            if line.is_empty() {
                // Blank line = end of event. An event with no `data:` field
                // (a bare `event:` or a keep-alive comment) yields nothing.
                if !self.data.is_empty() {
                    payloads.push(std::mem::take(&mut self.data));
                }
            } else if let Some(rest) = line.strip_prefix(SSE_DATA_FIELD) {
                if !self.data.is_empty() {
                    self.data.push('\n');
                }
                // The spec strips exactly ONE leading space after the colon.
                self.data.push_str(rest.strip_prefix(' ').unwrap_or(rest));
            }
            // Any other field (`event:`, `id:`, `retry:`, `:comment`) is
            // ignored: the payload JSON carries its own `type`, so the
            // `event:` line is redundant for us.
        }

        Ok(payloads)
    }
}

// ─────────────────────────────────────────────────────────────────
// Message accumulation
// ─────────────────────────────────────────────────────────────────

/// Whether the accumulator has seen the end of the message.
///
/// Returned by [`MessageAccumulator::push`] so the transport loop knows when to
/// stop reading the socket rather than waiting for the server to close it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    /// More events are expected.
    Continue,
    /// `message_stop` was seen; call [`MessageAccumulator::finish`].
    Done,
}

/// Rebuilds a [`StreamedMessage`] from decoded event payloads.
#[derive(Debug, Default)]
pub struct MessageAccumulator {
    /// Per-content-block text, indexed by the block index the provider assigns.
    /// `None` marks a block that is not a `text` block (tool use, thinking) and
    /// is therefore skipped when joining — mirroring the non-streaming path,
    /// which also collected only text blocks.
    blocks: Vec<Option<String>>,
    message_id: Option<String>,
    input_tokens: Option<u64>,
    /// Output tokens as first reported on `message_start` — a placeholder that
    /// `message_delta` normally supersedes. Kept as the fallback so a stream
    /// that ends without delta usage still reports SOMETHING rather than
    /// silently costing zero.
    start_output_tokens: Option<u64>,
    delta_output_tokens: Option<u64>,
    stop_reason: Option<String>,
    saw_message_stop: bool,
    events_seen: usize,
}

impl MessageAccumulator {
    /// Create an empty accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many events have been folded in so far. Read by the transport for
    /// its progress logging, and by [`StreamError::Incomplete`].
    pub fn events_seen(&self) -> usize {
        self.events_seen
    }

    /// Fold one decoded `data:` payload into the message under construction.
    ///
    /// # Errors
    ///
    /// - [`StreamError::MalformedEvent`] if the payload is not readable JSON.
    /// - [`StreamError::ProviderEvent`] if the payload is an `error` event.
    pub fn push(&mut self, payload: &str) -> Result<Progress, StreamError> {
        let event: WireEvent =
            serde_json::from_str(payload).map_err(|source| StreamError::MalformedEvent {
                preview: payload.chars().take(MALFORMED_PREVIEW_CHARS).collect(),
                source,
            })?;
        self.events_seen += 1;

        match event {
            WireEvent::MessageStart { message } => {
                self.message_id = Some(message.id);
                self.input_tokens = message.usage.input_tokens;
                self.start_output_tokens = message.usage.output_tokens;
            }
            WireEvent::ContentBlockStart {
                index,
                content_block,
            } => self.open_block(index, &content_block.kind),
            WireEvent::ContentBlockDelta { index, delta } => {
                if let WireContentDelta::TextDelta { text } = delta {
                    self.append_text(index, &text);
                }
            }
            WireEvent::MessageDelta { delta, usage } => {
                if delta.stop_reason.is_some() {
                    self.stop_reason = delta.stop_reason;
                }
                if usage.output_tokens.is_some() {
                    self.delta_output_tokens = usage.output_tokens;
                }
            }
            WireEvent::MessageStop => {
                self.saw_message_stop = true;
                return Ok(Progress::Done);
            }
            WireEvent::Error { error } => {
                return Err(StreamError::ProviderEvent {
                    kind: error.kind,
                    message: error.message,
                })
            }
            // `content_block_stop`, `ping`, and anything Anthropic adds later
            // carry nothing we accumulate.
            WireEvent::ContentBlockStop {} | WireEvent::Unknown => {}
        }

        Ok(Progress::Continue)
    }

    /// Consume the accumulator and produce the finished message.
    ///
    /// # Errors
    ///
    /// - [`StreamError::Incomplete`] if `message_stop` never arrived.
    /// - [`StreamError::MissingStopReason`] if it arrived but no `stop_reason`
    ///   ever did — see the module doc for why that is fatal rather than `None`.
    pub fn finish(self) -> Result<StreamedMessage, StreamError> {
        if !self.saw_message_stop {
            return Err(StreamError::Incomplete {
                events_seen: self.events_seen,
            });
        }
        let stop_reason = self.stop_reason.ok_or(StreamError::MissingStopReason)?;

        // ## Rust Learning: `flatten()` on an iterator of `Option<T>`
        //
        // `Vec<Option<String>>::into_iter().flatten()` yields only the `Some`
        // payloads and drops the `None`s — exactly "skip the non-text blocks".
        // The `\n` join reproduces what the non-streaming path did when it
        // concatenated a response's text blocks.
        let text = self
            .blocks
            .into_iter()
            .flatten()
            .collect::<Vec<String>>()
            .join("\n");

        Ok(StreamedMessage {
            text,
            input_tokens: self.input_tokens,
            output_tokens: self.delta_output_tokens.or(self.start_output_tokens),
            stop_reason,
            message_id: self.message_id,
        })
    }

    /// Open a content block, remembering whether it is one whose deltas we keep.
    ///
    /// A non-`text` block gets a `None` slot rather than no slot at all, so the
    /// block indices stay aligned and the join later skips it — exactly what the
    /// non-streaming path did when it collected only text blocks.
    fn open_block(&mut self, index: usize, kind: &str) {
        let slot = if kind == BLOCK_TYPE_TEXT {
            Some(String::new())
        } else {
            None
        };
        self.set_block(index, slot);
    }

    /// Append a `text_delta` to its block.
    ///
    /// A delta for a block we never saw start is not possible in a well-formed
    /// stream, but opening one on the fly is strictly safer than dropping prose
    /// on the floor: the alternative is a silently short extraction, which is
    /// the failure mode this whole module exists to prevent.
    fn append_text(&mut self, index: usize, text: &str) {
        if self.block_slot(index).is_none() {
            self.set_block(index, Some(String::new()));
        }
        if let Some(Some(buf)) = self.blocks.get_mut(index) {
            buf.push_str(text);
        }
    }

    /// Read a block slot without panicking on an out-of-range index.
    fn block_slot(&self, index: usize) -> Option<&String> {
        self.blocks.get(index).and_then(|slot| slot.as_ref())
    }

    /// Write a block slot, growing the vector as needed.
    ///
    /// Block indices arrive in order in practice, but the provider owns the
    /// numbering and an index-based write must not panic on a gap. Missing
    /// slots fill with `None`, which the join skips.
    fn set_block(&mut self, index: usize, value: Option<String>) {
        if self.blocks.len() <= index {
            self.blocks.resize_with(index + 1, || None);
        }
        // Bounds guaranteed by the resize above; `get_mut` rather than `[index]`
        // keeps the guarantee in the type system instead of in a comment.
        if let Some(slot) = self.blocks.get_mut(index) {
            *slot = value;
        }
    }
}

#[cfg(test)]
#[path = "anthropic_stream_tests.rs"]
mod tests;
