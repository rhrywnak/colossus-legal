//! The socket half of the streaming Messages call: issue the request, then
//! drive the response body through [`crate::pipeline::anthropic_stream`].
//!
//! ## Where the code went (CC_TASK_CHAT_ENGINE_v1, ruling D2)
//!
//! The driver, the status classifier and the chunk source now live in the shared
//! `colossus-chat` crate (`colossus_chat::transport`), generic over what the events
//! fold into. This module is the extraction engine's instantiation of it: the same
//! names at the same paths, fixed to [`StreamError`] and [`MessageAccumulator`], so
//! the engine and its tests read exactly as they did. The reasoning below still
//! describes the policy; it is enforced in the crate.
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

use std::time::Duration;

use crate::pipeline::anthropic_stream::{MessageAccumulator, StreamError, StreamedMessage};

pub use colossus_chat::transport::{
    classify_status as classify_status_generic, parse_retry_after, RejectionKind, ResponseChunks,
    RETRY_AFTER,
};

/// How much of a non-2xx response body is carried into the error message.
///
/// CONST: error-message ergonomics. Anthropic error bodies are small JSON
/// objects; the cap exists so an HTML error page from an intercepting proxy
/// cannot flood `pipeline_jobs.error`. (The extraction engine's own value, as it
/// was before the transport moved to `colossus-chat`; the crate takes it as an
/// argument, and the chat engine's comes from its settings row.)
pub const ERROR_BODY_PREVIEW_CHARS: usize = 500;

/// Truncate a response body for an error message, at this engine's length.
pub fn preview_body(body: &str) -> String {
    colossus_chat::transport::preview_body(body, ERROR_BODY_PREVIEW_CHARS)
}

/// Everything the transport can fail with, fixed to the extraction stream's
/// error type.
///
/// ## Rust Learning: a type alias over a generic enum
///
/// `TransportError::IdleTimeout { .. }` still works as a pattern and a
/// constructor, because Rust resolves enum variants through a type alias. That is
/// what let the enum become generic in the crate while every `match` here stayed
/// word-for-word the same.
pub type TransportError = colossus_chat::transport::TransportError<StreamError>;

/// Classify a non-success HTTP status — the crate's classifier at this module's
/// error type. See `colossus_chat::transport::classify_status`.
pub fn classify_status(status: u16, retry_after: Option<&str>, body: &str) -> TransportError {
    classify_status_generic(status, retry_after, body, ERROR_BODY_PREVIEW_CHARS)
}

/// The source of response-body bytes that [`drive`] reads, at this module's
/// error type.
///
/// Kept as its own trait (rather than a re-export of the crate's generic one) so a
/// caller writes `impl ChunkSource for X` with no type parameter, exactly as
/// before D2. [`Adapt`] bridges it to the crate's trait.
#[async_trait::async_trait]
pub trait ChunkSource: Send {
    /// Yield the next slice of response body, or `None` at end of body.
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, TransportError>;
}

#[async_trait::async_trait]
impl ChunkSource for ResponseChunks {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, TransportError> {
        colossus_chat::transport::ChunkSource::<StreamError>::next_chunk(self).await
    }
}

/// Wraps a borrowed local [`ChunkSource`] so the crate's driver can read it.
///
/// ## Rust Learning: the newtype adapter
///
/// Rust forbids implementing a foreign trait for a foreign type, and a blanket
/// `impl<T: ChunkSource> colossus_chat::…::ChunkSource for T` would collide with
/// the crate's own impl for `ResponseChunks`. A one-field struct we OWN sidesteps
/// both rules: the impl is for `Adapt`, which is local.
struct Adapt<'a, S: ?Sized>(&'a mut S);

#[async_trait::async_trait]
impl<S: ChunkSource + ?Sized> colossus_chat::transport::ChunkSource<StreamError> for Adapt<'_, S> {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, TransportError> {
        self.0.next_chunk().await
    }
}

/// Drive a chunk source to a finished message, failing on an idle gap.
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
    colossus_chat::transport::drive(
        &mut Adapt(source),
        MessageAccumulator::new(),
        idle_timeout,
        // The extraction engine consumes the finished message only; there is no
        // partial state to forward.
        |_fold: &mut MessageAccumulator| {},
    )
    .await
}

#[cfg(test)]
#[path = "anthropic_transport_tests.rs"]
mod tests;
