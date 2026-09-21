//! The socket half of a streaming Messages call: classify the response status,
//! then drive the body through [`crate::sse::SseDecoder`] into an [`EventFold`].
//!
//! Moved here from `colossus-legal`'s `pipeline::anthropic_transport`
//! (CC_TASK_CHAT_ENGINE_v1, ruling D2). The only change is that the thing the
//! events fold INTO is now a type parameter: the extraction engine folds into its
//! text-only accumulator, the chat engine folds into a full-fidelity content-block
//! accumulator, and both share this one time-and-framing policy.
//!
//! ## Why there is no whole-request timeout
//!
//! The 2026-08-28 incident was a `reqwest` client built with a 600-second total
//! timeout. That timeout covers the entire exchange — connect, send, and read the
//! body to completion — so it fired while Opus 5 was healthily producing a
//! 64000-token answer. A total-duration cap is wrong *by design* for a streaming
//! response: a long answer is not a symptom, it is the product.
//!
//! What a broken stream actually looks like is a GAP. A healthy Anthropic
//! stream emits events continuously (and `ping` events even while the model is
//! thinking), so "no event for N seconds" is the real failure signal. That is
//! what [`drive`] enforces, and it is the only time-based limit on the call.
//!
//! ## Domain note: the clock resets on EVENTS, not on bytes
//!
//! Resetting an idle timer whenever bytes arrive would be weaker than it looks:
//! a half-delivered line, or a stream dribbling out a framing artefact, would
//! keep the timer alive forever without the message ever advancing. The clock
//! here is reset only when the decoder has completed a whole event and the fold
//! has accepted it — so the timeout measures progress on the MESSAGE.
//!
//! ## Connect timeout survives
//!
//! Standing Rule 13 requires every HTTP call to have a timeout. A streaming call
//! cannot honour a total-duration one, so the compensating controls are the
//! connect timeout (a dead host still fails fast) and the idle timeout above.

use std::time::{Duration, Instant};

use crate::sse::{SseDecoder, SseError};

/// HTTP status meaning "rate limited" (`rate_limit_error`).
///
/// CONST: an HTTP status code — protocol vocabulary, not a setting.
const STATUS_TOO_MANY_REQUESTS: u16 = 429;

/// HTTP status Anthropic returns for `overloaded_error`.
///
/// CONST: Anthropic's documented overload status. Grouped with 429 because both
/// mean the SAME operational thing — the request was refused at the front door
/// and no generation began. See [`RejectionKind`].
const STATUS_OVERLOADED: u16 = 529;

/// How much of a non-2xx response body is carried into the error message.
///
/// CONST: error-message ergonomics. Anthropic error bodies are small JSON
/// objects; the cap exists so an HTML error page from an intercepting proxy
/// cannot flood a stored error column.
pub const ERROR_BODY_PREVIEW_CHARS: usize = 500;

/// The response header carrying the provider's requested backoff, in seconds.
/// Exposed so the one spelling lives in one place.
pub const RETRY_AFTER: &str = "retry-after";

/// Whether a fold has seen the end of the message.
///
/// Returned by [`EventFold::push`] so [`drive`] knows when to stop reading the
/// socket rather than waiting for the server to close it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    /// More events are expected.
    Continue,
    /// `message_stop` was seen; [`EventFold::finish`] may be called.
    Done,
}

/// Something that turns decoded event payloads into a finished value.
///
/// ## Rust Learning: associated types vs generic parameters on a trait
///
/// `type Output` and `type Error` are *associated types*: each implementor picks
/// exactly one of each. A generic parameter (`trait EventFold<O, E>`) would let one
/// type implement the trait many times over, which is not a thing a fold can
/// meaningfully do — the extraction accumulator produces one kind of message and
/// fails one kind of way. Associated types say that in the type system and spare
/// every caller from naming them.
///
/// `Error: From<SseError>` is what lets [`drive`] use `?` on a framing failure and
/// have it land inside the fold's own error type, so the caller sees ONE taxonomy.
pub trait EventFold {
    /// The finished value.
    type Output;
    /// Everything folding can fail with.
    type Error: From<SseError>;

    /// Fold one decoded `data:` payload in.
    ///
    /// # Errors
    /// Whatever the implementor refuses: malformed JSON, an `error` event, …
    fn push(&mut self, payload: &str) -> Result<Progress, Self::Error>;

    /// How many events have been folded in — for the idle-timeout message.
    fn events_seen(&self) -> usize;

    /// Consume the fold and produce the finished value.
    ///
    /// # Errors
    /// An incomplete or otherwise unusable message.
    fn finish(self) -> Result<Self::Output, Self::Error>;
}

/// Which pre-generation refusal the provider returned.
///
/// The two are handled identically by any retry policy; the distinction is kept
/// so the log and the failure message say which one happened. "Rate limited" and
/// "the provider is overloaded" call for different operator responses — the
/// first is usually our own concurrency, the second is never anything we did —
/// and collapsing them would hide that (Standing Rule 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectionKind {
    /// HTTP 429 — `rate_limit_error`. Our quota, usually our own concurrency.
    RateLimited,
    /// HTTP 529 — `overloaded_error`. Anthropic's capacity, not ours.
    Overloaded,
}

/// ## Rust Learning: `Display` vs the derived `Debug` on a small enum
///
/// `Debug` prints the Rust identifier — `RateLimited`. This impl is what an
/// operator reads in a stored error, so it spells out the status code and
/// Anthropic's own error-type name: the two things worth pasting into a support
/// conversation.
impl std::fmt::Display for RejectionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RateLimited => write!(f, "HTTP 429 rate_limit_error"),
            Self::Overloaded => write!(f, "HTTP 529 overloaded_error"),
        }
    }
}

/// Everything the transport can fail with.
///
/// ## Rust Learning: a generic error enum
///
/// `E` is the fold's own error type (`Stream(E)`). The extraction engine
/// instantiates this as `TransportError<StreamError>`, the chat engine as
/// `TransportError<ChatStreamError>`; the four transport-level variants are shared.
/// Keeping `Stream` separate answers a different question: `E` is "the message
/// the provider sent was not well-formed", the rest are "the exchange did not
/// happen".
#[derive(Debug, thiserror::Error)]
pub enum TransportError<E: std::error::Error + 'static> {
    /// The request could not be sent, or the body could not be read.
    #[error("the streaming request to the Anthropic Messages API failed: {source}")]
    Http {
        /// The underlying reqwest failure.
        #[source]
        source: reqwest::Error,
    },

    /// A non-2xx response other than 429 / 529.
    #[error("the Anthropic Messages API returned HTTP {status}: {body}")]
    Status {
        /// The HTTP status code.
        status: u16,
        /// The response body, truncated to [`ERROR_BODY_PREVIEW_CHARS`].
        body: String,
    },

    /// HTTP 429 or 529 — the request was REJECTED before generation began.
    ///
    /// ## Domain note: why these two are the only free retries
    ///
    /// Both statuses are refusals at the front door: the model never started, so
    /// nothing was billed and retrying costs only wall-clock. The exemption is
    /// earned by WHERE the failure was detected, not by what it is called: an
    /// `overloaded_error` arriving as an event inside an already-open stream
    /// reaches [`TransportError::Stream`] instead, because by then generation may
    /// have started and may have billed.
    #[error(
        "the Anthropic Messages API rejected the request before generation: \
         {kind} (retry-after: {retry_after_secs:?})"
    )]
    Rejected {
        /// Which refusal it was, for the operator-facing message.
        kind: RejectionKind,
        /// Seconds the provider asked us to wait, if it said.
        retry_after_secs: Option<u64>,
    },

    /// No event completed within the idle window.
    #[error(
        "no server-sent event arrived for {idle_secs}s (after {events_seen} events) — \
         the stream is stalled and the call is abandoned. A healthy Anthropic stream \
         emits events continuously, so this is a dropped or wedged connection, not a \
         slow model. Raise LLM_STREAM_IDLE_TIMEOUT_SECS only if the network genuinely \
         buffers longer than this"
    )]
    IdleTimeout {
        /// The configured idle window, in seconds.
        idle_secs: u64,
        /// How many events had been folded in before the stall.
        events_seen: usize,
    },

    /// The bytes arrived but did not form a well-formed message.
    #[error("{0}")]
    Stream(E),
}

/// Read a `retry-after` value out of response headers.
///
/// Anthropic sends an integer number of seconds. A malformed or absent header
/// yields `None`, which the caller treats as "the provider did not say" — a
/// distinct state from `Some(0)` (Standing Rule 1).
pub fn parse_retry_after(raw: Option<&str>) -> Option<u64> {
    raw.and_then(|value| value.trim().parse::<u64>().ok())
}

/// Truncate a response body for inclusion in an error message.
pub fn preview_body(body: &str) -> String {
    body.chars().take(ERROR_BODY_PREVIEW_CHARS).collect()
}

/// Classify a non-success HTTP status into a [`TransportError`].
///
/// 429 and 529 become the typed [`TransportError::Rejected`] variant carrying
/// whatever `retry-after` the provider sent; everything else becomes `Status`.
/// This function is the ONLY place a pre-generation rejection is recognised, and
/// it runs before a single body byte is read.
pub fn classify_status<E: std::error::Error + 'static>(
    status: u16,
    retry_after: Option<&str>,
    body: &str,
) -> TransportError<E> {
    let kind = match status {
        STATUS_TOO_MANY_REQUESTS => Some(RejectionKind::RateLimited),
        STATUS_OVERLOADED => Some(RejectionKind::Overloaded),
        _ => None,
    };
    match kind {
        Some(kind) => TransportError::Rejected {
            kind,
            retry_after_secs: parse_retry_after(retry_after),
        },
        None => TransportError::Status {
            status,
            body: preview_body(body),
        },
    }
}

/// The source of response-body bytes that [`drive`] reads.
///
/// ## Rust Learning: why a trait and not a closure
///
/// The obvious shape — `drive(|| async { response.chunk().await })` — does not
/// compile: an `FnMut` closure may not return a future that borrows a captured
/// variable, because the borrow would outlive the closure call. A trait with
/// `&mut self` sidesteps it: the borrow is a parameter of the call rather than a
/// capture, so it lives exactly as long as the future does. `#[async_trait]`
/// boxes that future — one small allocation per socket chunk, invisible next to
/// the LLM round-trip it is reading. The indirection is what lets a test supply
/// canned bytes instead of a socket.
#[async_trait::async_trait]
pub trait ChunkSource<E: std::error::Error + Send + 'static>: Send {
    /// Yield the next slice of response body, or `None` at end of body.
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, TransportError<E>>;
}

/// [`ChunkSource`] over a live `reqwest` response.
pub struct ResponseChunks {
    /// The streaming response, read incrementally.
    response: reqwest::Response,
}

impl ResponseChunks {
    /// Wrap a response whose status has already been checked.
    pub fn new(response: reqwest::Response) -> Self {
        Self { response }
    }
}

#[async_trait::async_trait]
impl<E: std::error::Error + Send + 'static> ChunkSource<E> for ResponseChunks {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, TransportError<E>> {
        self.response
            .chunk()
            .await
            // One copy per socket chunk — far below the cost of the call itself —
            // and an owned `Vec` is what lets a test supply chunks from a string
            // literal through the same trait.
            .map(|opt| opt.map(|bytes| bytes.to_vec()))
            .map_err(|source| TransportError::Http { source })
    }
}

/// Drive a chunk source through a fold to a finished value, failing on an idle gap.
///
/// This function owns the only time-based limit on a streaming call. There is
/// deliberately no total-duration cap — see the module doc.
///
/// `on_event` is called after every accepted event with the fold, so a caller
/// can forward partial state (the chat engine streams text deltas to the browser
/// this way). The extraction engine passes a no-op.
///
/// # Errors
///
/// - [`TransportError::IdleTimeout`] when no event completes inside the window.
/// - [`TransportError::Stream`] for a malformed, errored, or incomplete message.
/// - Whatever the source yields for a read failure.
pub async fn drive<S, F, C>(
    source: &mut S,
    mut fold: F,
    idle_timeout: Duration,
    mut on_event: C,
) -> Result<F::Output, TransportError<F::Error>>
where
    F: EventFold,
    F::Error: std::error::Error + Send + 'static,
    S: ChunkSource<F::Error> + ?Sized,
    C: FnMut(&mut F),
{
    let idle_secs = idle_timeout.as_secs();
    let mut decoder = SseDecoder::new();
    // The idle clock. Reset when an EVENT is folded in — see the module doc.
    let mut last_event = Instant::now();

    loop {
        // Saturating subtraction: an elapsed time past the window yields zero
        // rather than panicking on underflow, and zero is handled immediately.
        let remaining = idle_timeout.saturating_sub(last_event.elapsed());
        if remaining.is_zero() {
            return Err(TransportError::IdleTimeout {
                idle_secs,
                events_seen: fold.events_seen(),
            });
        }

        let chunk = match tokio::time::timeout(remaining, source.next_chunk()).await {
            // The timeout fired: no bytes at all inside the remaining window.
            Err(_elapsed) => {
                return Err(TransportError::IdleTimeout {
                    idle_secs,
                    events_seen: fold.events_seen(),
                })
            }
            Ok(Err(e)) => return Err(e),
            // End of body. A complete message would already have returned on
            // `Progress::Done`, so reaching here means the server closed early —
            // `finish` turns that into the fold's own "incomplete" error.
            Ok(Ok(None)) => break,
            Ok(Ok(Some(bytes))) => bytes,
        };

        let payloads = decoder
            .push_bytes(&chunk)
            .map_err(|e| TransportError::Stream(F::Error::from(e)))?;
        for payload in payloads {
            let progress = fold.push(&payload).map_err(TransportError::Stream)?;
            // Reset AFTER a successful fold: the message advanced.
            last_event = Instant::now();
            on_event(&mut fold);
            if progress == Progress::Done {
                return fold.finish().map_err(TransportError::Stream);
            }
        }
    }

    fold.finish().map_err(TransportError::Stream)
}
