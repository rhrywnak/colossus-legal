//! The socket half of the streaming Messages call: issue the request, then
//! drive the response body through [`crate::pipeline::anthropic_stream`].
//!
//! ## Why there is no whole-request timeout any more
//!
//! The 2026-08-28 incident was a `reqwest` client built with
//! `.timeout(Duration::from_secs(600))`. That timeout covers the entire
//! exchange — connect, send, and read the body to completion — so it fired
//! while Opus 5 was healthily producing a 64000-token answer. A total-duration
//! cap is wrong *by design* for a streaming response: a long answer is not a
//! symptom, it is the product.
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
//! here is reset only when [`crate::pipeline::anthropic_stream::SseDecoder`] has
//! completed a whole event and the accumulator has folded it in — so the timeout
//! measures progress on the MESSAGE, which is the thing we care about.
//!
//! ## Connect timeout survives
//!
//! Standing Rule 13 requires every HTTP call to have a timeout. A streaming call
//! cannot honour a total-duration one, so the compensating controls are the
//! connect timeout (a dead host still fails fast) and the idle timeout above.
//! Both are configured in [`crate::pipeline::anthropic_engine`].

use std::time::{Duration, Instant};

use crate::pipeline::anthropic_stream::{
    MessageAccumulator, Progress, SseDecoder, StreamError, StreamedMessage,
};

/// HTTP status meaning "rate limited".
///
/// CONST: an HTTP status code — protocol vocabulary, not a setting.
const STATUS_TOO_MANY_REQUESTS: u16 = 429;

/// Response header carrying the provider's requested backoff, in seconds.
const RETRY_AFTER_HEADER: &str = "retry-after";

/// How much of a non-2xx response body is carried into the error message.
///
/// CONST: error-message ergonomics. Anthropic error bodies are small JSON
/// objects; the cap exists so an HTML error page from an intercepting proxy
/// cannot flood `pipeline_jobs.error`.
const ERROR_BODY_PREVIEW_CHARS: usize = 500;

/// Everything the transport can fail with.
///
/// Kept separate from [`StreamError`] because the two answer different
/// questions: `StreamError` is "the message the provider sent was not
/// well-formed", while these are "the exchange did not happen". The adapter
/// above maps both into the engine's error taxonomy.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// The request could not be sent, or the body could not be read.
    #[error("the streaming request to the Anthropic Messages API failed: {source}")]
    Http {
        /// The underlying reqwest failure.
        #[source]
        source: reqwest_13::Error,
    },

    /// A non-2xx response other than 429.
    #[error("the Anthropic Messages API returned HTTP {status}: {body}")]
    Status {
        /// The HTTP status code.
        status: u16,
        /// The response body, truncated to [`ERROR_BODY_PREVIEW_CHARS`].
        body: String,
    },

    /// HTTP 429. Carries the provider's own `retry-after` when it sent one —
    /// which, unlike the previous Rig-based transport, we can now read, because
    /// we hold the response headers ourselves.
    #[error(
        "the Anthropic Messages API rate-limited the request (retry-after: {retry_after_secs:?})"
    )]
    RateLimited {
        /// Seconds the provider asked us to wait, if it said.
        retry_after_secs: Option<u64>,
    },

    /// No event completed within the idle window. This is the replacement for
    /// the 600s whole-request timeout that caused the 2026-08-28 incident.
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
    Stream(#[from] StreamError),
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
/// Split out from the request path so both branches are assertable without a
/// socket: 429 becomes the typed rate-limit variant carrying whatever
/// `retry-after` the provider sent, everything else becomes `Status`.
pub fn classify_status(status: u16, retry_after: Option<&str>, body: &str) -> TransportError {
    if status == STATUS_TOO_MANY_REQUESTS {
        return TransportError::RateLimited {
            retry_after_secs: parse_retry_after(retry_after),
        };
    }
    TransportError::Status {
        status,
        body: preview_body(body),
    }
}

/// The header name the caller reads off the response before calling
/// [`classify_status`]. Exposed so the one spelling lives in one place.
pub const RETRY_AFTER: &str = RETRY_AFTER_HEADER;

/// The source of response-body bytes that [`drive`] reads.
///
/// ## Rust Learning: why a trait and not a closure
///
/// The obvious shape — `drive(|| async { response.chunk().await })` — does not
/// compile: an `FnMut` closure may not return a future that borrows a captured
/// variable, because the borrow would outlive the closure call. Wrapping the
/// response in a `RefCell` gets past the borrow checker but produces a
/// non-`Send` future (a `RefMut` held across an `.await`), and `ExtractionEngine`
/// requires `Send`.
///
/// A trait with `&mut self` sidesteps both: the borrow is a parameter of the
/// call rather than a capture, so it lives exactly as long as the future does.
/// `#[async_trait]` boxes that future — one small allocation per socket chunk,
/// invisible next to the LLM round-trip it is reading.
///
/// The point of the indirection is testability: [`drive`] owns the entire
/// time-and-framing policy, and a test supplies canned bytes instead of a
/// socket. That is what lets the idle timeout be asserted with no Anthropic
/// call and no tokens spent.
#[async_trait::async_trait]
pub trait ChunkSource: Send {
    /// Yield the next slice of response body, or `None` at end of body.
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, TransportError>;
}

/// [`ChunkSource`] over a live `reqwest` response.
pub struct ResponseChunks {
    /// The streaming response, read incrementally.
    response: reqwest_13::Response,
}

impl ResponseChunks {
    /// Wrap a response whose status has already been checked.
    pub fn new(response: reqwest_13::Response) -> Self {
        Self { response }
    }
}

#[async_trait::async_trait]
impl ChunkSource for ResponseChunks {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, TransportError> {
        self.response
            .chunk()
            .await
            // One copy per socket chunk. Measured against a 64000-token response
            // this is a few hundred kilobytes in total — far below the cost of
            // the call itself — and an owned `Vec` is what lets a test supply
            // chunks from a string literal through the same trait.
            .map(|opt| opt.map(|bytes| bytes.to_vec()))
            .map_err(|source| TransportError::Http { source })
    }
}

/// Drive a chunk source to a finished message, failing on an idle gap.
///
/// This function owns the only time-based limit on a streaming call. There is
/// deliberately no total-duration cap — see the module doc.
///
/// # Errors
///
/// - [`TransportError::IdleTimeout`] when no event completes inside the window.
/// - [`TransportError::Stream`] for a malformed, errored, or incomplete message.
/// - Whatever the source yields for a read failure.
pub async fn drive<S: ChunkSource + ?Sized>(
    source: &mut S,
    idle_timeout: Duration,
) -> Result<StreamedMessage, TransportError> {
    let idle_secs = idle_timeout.as_secs();
    let mut decoder = SseDecoder::new();
    let mut accumulator = MessageAccumulator::new();
    // The idle clock. Reset when an EVENT is folded in — see the module doc for
    // why bytes are not enough.
    let mut last_event = Instant::now();

    loop {
        // Remaining budget before the stream counts as stalled. Saturating
        // subtraction: an elapsed time past the window yields zero rather than
        // panicking on underflow, and zero is handled immediately below.
        let remaining = idle_timeout.saturating_sub(last_event.elapsed());
        if remaining.is_zero() {
            return Err(TransportError::IdleTimeout {
                idle_secs,
                events_seen: accumulator.events_seen(),
            });
        }

        let chunk = match tokio::time::timeout(remaining, source.next_chunk()).await {
            // The timeout fired: no bytes at all inside the remaining window.
            Err(_elapsed) => {
                return Err(TransportError::IdleTimeout {
                    idle_secs,
                    events_seen: accumulator.events_seen(),
                })
            }
            Ok(Err(e)) => return Err(e),
            // End of body. A complete message would already have returned on
            // `Progress::Done`, so reaching here means the server closed early —
            // `finish` turns that into `Incomplete`.
            Ok(Ok(None)) => break,
            Ok(Ok(Some(bytes))) => bytes,
        };

        for payload in decoder.push_bytes(&chunk)? {
            let progress = accumulator.push(&payload)?;
            // Reset AFTER a successful fold: the message advanced.
            last_event = Instant::now();
            if progress == Progress::Done {
                return Ok(accumulator.finish()?);
            }
        }
    }

    Ok(accumulator.finish()?)
}

#[cfg(test)]
#[path = "anthropic_transport_tests.rs"]
mod tests;
