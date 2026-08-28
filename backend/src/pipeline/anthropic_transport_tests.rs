//! Unit tests for the stream driver's timing and framing policy.
//!
//! Every test supplies its own chunk source, so the idle timeout — the control
//! that replaces the 600s whole-request timeout that caused the 2026-08-28
//! incident — is asserted with no socket, no Anthropic call, and no tokens
//! spent.
//!
//! ## Rust Learning: `tokio::time::pause()`
//!
//! Asserting a 120-second timeout by waiting 120 seconds would make the suite
//! useless. `pause()` switches the tokio runtime to a virtual clock that only
//! advances when every task is idle — so a `tokio::time::timeout` that has
//! nothing to wait for fires instantly, in real time, at exactly the virtual
//! deadline. `start_paused = true` on the test attribute pauses before the body
//! runs. This is the same mechanism the runtime's own timer tests use.

use super::*;

/// A minimal complete transcript — enough to reach `message_stop`.
const COMPLETE: &str = concat!(
    r#"data: {"type":"message_start","message":{"id":"msg_t1","usage":{"input_tokens":7,"output_tokens":1}}}"#,
    "\n\n",
    r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
    "\n\n",
    r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"done"}}"#,
    "\n\n",
    r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":4}}"#,
    "\n\n",
    r#"data: {"type":"message_stop"}"#,
    "\n\n",
);

/// A [`ChunkSource`] over canned bytes, optionally pausing before each chunk.
///
/// `gap` is what makes the two timing tests possible without a socket: a source
/// that pauses ten minutes between events, and a source that pauses forever
/// after the first one, are the healthy-but-slow and stalled cases respectively.
struct CannedChunks {
    /// Remaining chunks, front first.
    chunks: std::collections::VecDeque<Vec<u8>>,
    /// How long to wait before yielding each chunk.
    gap: Duration,
    /// When the queue empties: `true` blocks forever (a stalled connection),
    /// `false` reports end-of-body (a closed connection).
    hang_when_empty: bool,
}

impl CannedChunks {
    fn new(pieces: &[&str]) -> Self {
        Self {
            chunks: pieces.iter().map(|s| s.as_bytes().to_vec()).collect(),
            gap: Duration::ZERO,
            hang_when_empty: false,
        }
    }

    /// Split a transcript into one chunk per SSE event.
    fn per_event(transcript: &str) -> Self {
        let pieces: Vec<&str> = transcript.split_inclusive("\n\n").collect();
        Self::new(&pieces)
    }

    fn with_gap(mut self, gap: Duration) -> Self {
        self.gap = gap;
        self
    }

    fn hanging(mut self) -> Self {
        self.hang_when_empty = true;
        self
    }
}

#[async_trait::async_trait]
impl ChunkSource for CannedChunks {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, TransportError> {
        if !self.gap.is_zero() {
            tokio::time::sleep(self.gap).await;
        }
        match self.chunks.pop_front() {
            Some(chunk) => Ok(Some(chunk)),
            None if self.hang_when_empty => {
                // A connection that went quiet without closing — exactly the
                // shape a total-duration timeout used to "handle" by killing
                // healthy calls too. The virtual clock makes this instant.
                tokio::time::sleep(Duration::from_secs(86_400)).await;
                Ok(None)
            }
            None => Ok(None),
        }
    }
}

#[tokio::test]
async fn a_complete_stream_delivered_in_pieces_produces_the_message() {
    // The transcript is ASCII, so splitting it at an arbitrary byte is safe and
    // is the hostile framing a socket would actually produce.
    let (head, tail) = COMPLETE.split_at(COMPLETE.len() / 2);
    let mut source = CannedChunks::new(&[head, tail]);
    let message = drive(&mut source, Duration::from_secs(30))
        .await
        .expect("a complete stream must drive to a message");
    assert_eq!(message.text, "done");
    assert_eq!(message.stop_reason, "end_turn");
    assert_eq!(message.input_tokens, Some(7));
    assert_eq!(message.output_tokens, Some(4));
}

#[tokio::test(start_paused = true)]
async fn the_driver_stops_at_message_stop_without_waiting_for_the_socket_to_close() {
    // The source never reports end-of-body. If the driver waited for the server
    // to close it would hit the idle timeout instead of returning — so this
    // asserts that `Progress::Done` ends the read.
    let mut source = CannedChunks::new(&[COMPLETE]).hanging();
    let message = drive(&mut source, Duration::from_secs(120))
        .await
        .expect("message_stop must end the read");
    assert_eq!(message.text, "done");
}

#[tokio::test(start_paused = true)]
async fn a_stalled_stream_fails_on_the_idle_gap_and_names_the_window() {
    // The failure the 600s whole-request timeout should have been all along: no
    // EVENT for the configured window. One valid opening event, then silence.
    let opening = concat!(
        r#"data: {"type":"message_start","message":{"id":"m","usage":{"input_tokens":1}}}"#,
        "\n\n"
    );
    let mut source = CannedChunks::new(&[opening]).hanging();
    match drive(&mut source, Duration::from_secs(120)).await {
        Err(TransportError::IdleTimeout {
            idle_secs,
            events_seen,
        }) => {
            assert_eq!(
                idle_secs, 120,
                "the failure must name the configured window"
            );
            assert_eq!(
                events_seen, 1,
                "the failure must say how far the message got before stalling"
            );
        }
        other => panic!("expected IdleTimeout, got {other:?}"),
    }
}

#[tokio::test(start_paused = true)]
async fn a_long_but_healthy_stream_is_never_cut_off_by_a_total_duration_cap() {
    // The incident, inverted. Each event arrives well inside the idle window,
    // but the whole exchange runs far longer than the 600s whole-request timeout
    // that failed the 36-page transcript on 2026-08-28. The old client killed
    // this call; this one must not.
    let mut source = CannedChunks::per_event(COMPLETE).with_gap(Duration::from_secs(600));
    let message = drive(&mut source, Duration::from_secs(1200))
        .await
        .expect("a slow but progressing stream must NOT be abandoned");
    assert_eq!(message.text, "done");
}

#[tokio::test]
async fn a_body_that_ends_before_message_stop_is_incomplete() {
    let partial = COMPLETE
        .split(r#"data: {"type":"message_delta""#)
        .next()
        .expect("split always yields a first element");
    let mut source = CannedChunks::new(&[partial]);
    match drive(&mut source, Duration::from_secs(30)).await {
        Err(TransportError::Stream(StreamError::Incomplete { .. })) => {}
        other => panic!("expected Stream(Incomplete), got {other:?}"),
    }
}

#[test]
fn a_429_is_classified_as_a_rejection_with_the_providers_own_retry_after() {
    // The Rig transport discarded this header and the bridge substituted 60s for
    // every rate limit. Holding the response ourselves gets the real value back.
    match classify_status(429, Some("17"), "{\"type\":\"error\"}") {
        TransportError::Rejected {
            kind,
            retry_after_secs,
        } => {
            assert_eq!(kind, RejectionKind::RateLimited);
            assert_eq!(retry_after_secs, Some(17));
        }
        other => panic!("expected Rejected, got {other:?}"),
    }
}

#[test]
fn a_529_is_classified_as_a_rejection_too() {
    // The amendment: HTTP 529 `overloaded_error` is a refusal at the front door
    // exactly like a 429, so it earns the same free-retry treatment. Anthropic
    // usually sends no retry-after with it, which is what the backoff schedule
    // in `llm_retry_policy` exists for.
    match classify_status(529, None, "{\"type\":\"error\"}") {
        TransportError::Rejected {
            kind,
            retry_after_secs,
        } => {
            assert_eq!(kind, RejectionKind::Overloaded);
            assert_eq!(retry_after_secs, None);
        }
        other => panic!("expected Rejected, got {other:?}"),
    }
}

#[test]
fn the_two_rejection_kinds_stay_distinguishable_in_what_an_operator_reads() {
    // They are handled identically, but "I hit my own quota" and "Anthropic is
    // out of capacity" call for different operator responses, so the message
    // must not collapse them (Standing Rule 1).
    assert!(RejectionKind::RateLimited.to_string().contains("429"));
    assert!(RejectionKind::Overloaded.to_string().contains("529"));
    assert_ne!(
        RejectionKind::RateLimited.to_string(),
        RejectionKind::Overloaded.to_string()
    );
}

#[test]
fn a_rejection_with_no_usable_retry_after_reports_none_not_zero() {
    // `None` means "the provider did not say" and selects the backoff schedule;
    // `Some(0)` would mean "retry immediately". Collapsing them would make the
    // schedule unreachable for a header-less 429 (Standing Rule 1).
    for status in [429u16, 529] {
        for header in [None, Some("soon"), Some("")] {
            match classify_status(status, header, "") {
                TransportError::Rejected {
                    retry_after_secs, ..
                } => {
                    assert_eq!(retry_after_secs, None, "status {status}, header {header:?}");
                }
                other => panic!("expected Rejected, got {other:?}"),
            }
        }
    }
}

#[test]
fn an_ordinary_server_error_is_not_a_rejection() {
    // The exemption must stay narrow. A 500 or a 503 says nothing about whether
    // generation began, so it does not get free retries.
    for status in [400u16, 401, 500, 503, 502] {
        match classify_status(status, None, "boom") {
            TransportError::Status { status: got, .. } => assert_eq!(got, status),
            other => panic!("status {status} must not be a rejection, got {other:?}"),
        }
    }
}

#[tokio::test]
// The capital is the point: WHERE the error was detected is what decides
// whether retrying it is free.
#[allow(non_snake_case)]
async fn an_overloaded_error_INSIDE_the_stream_is_never_a_rejection() {
    // The boundary the whole amendment turns on. The same provider condition —
    // "overloaded" — is free when it arrives as an HTTP status before the body,
    // and potentially already-billed when it arrives as an event after the
    // stream opened. Only `classify_status` can produce `Rejected`; the
    // accumulator's path cannot reach it, and this pins that structurally.
    let mid_stream = concat!(
        r#"data: {"type":"message_start","message":{"id":"m","usage":{"input_tokens":9}}}"#,
        "\n\n",
        r#"data: {"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#,
        "\n\n",
    );
    let mut source = CannedChunks::new(&[mid_stream]);
    match drive(&mut source, Duration::from_secs(30)).await {
        Err(TransportError::Stream(StreamError::ProviderEvent { kind, .. })) => {
            assert_eq!(kind, "overloaded_error");
        }
        other => panic!("expected Stream(ProviderEvent), got {other:?}"),
    }
}

#[test]
fn a_non_rejection_failure_keeps_its_status_and_a_bounded_body() {
    // 500 rather than 529: since the amendment, 529 is a rejection and takes the
    // free-retry path, so it is no longer an example of "everything else".
    let huge = "x".repeat(10_000);
    match classify_status(500, None, &huge) {
        TransportError::Status { status, body } => {
            assert_eq!(status, 500);
            assert_eq!(
                body.chars().count(),
                super::ERROR_BODY_PREVIEW_CHARS,
                "an oversized error body must be capped before it reaches pipeline_jobs.error"
            );
        }
        other => panic!("expected Status, got {other:?}"),
    }
}
