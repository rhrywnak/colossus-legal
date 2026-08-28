//! Unit tests for the SSE framer and the message accumulator.
//!
//! ## No live API calls, by ruling (2026-08-28)
//!
//! Money has been burned twice this week on retried Anthropic calls. Every
//! assertion here runs against a byte-string transcript of the streaming
//! protocol — the whole reason this module was kept pure. Nothing in this file
//! opens a socket, and nothing in it costs a token.
//!
//! The transcripts are the real wire shape: `event:` lines the framer must
//! ignore, `ping` events, a `content_block_stop`, and the `message_delta` that
//! carries `stop_reason`.

use super::*;

/// A minimal but faithful transcript of a successful streamed extraction.
///
/// The blank line between events is load-bearing — it is what terminates an
/// event in the `text/event-stream` format.
const HAPPY_TRANSCRIPT: &str = concat!(
    "event: message_start\n",
    r#"data: {"type":"message_start","message":{"id":"msg_01Test","type":"message","role":"assistant","model":"claude-opus-5","content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":41255,"output_tokens":1}}}"#,
    "\n\n",
    "event: content_block_start\n",
    r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
    "\n\n",
    "event: ping\n",
    r#"data: {"type":"ping"}"#,
    "\n\n",
    "event: content_block_delta\n",
    r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"{\"entities\":"}}"#,
    "\n\n",
    "event: content_block_delta\n",
    r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"[]}"}}"#,
    "\n\n",
    "event: content_block_stop\n",
    r#"data: {"type":"content_block_stop","index":0}"#,
    "\n\n",
    "event: message_delta\n",
    r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":18}}"#,
    "\n\n",
    "event: message_stop\n",
    r#"data: {"type":"message_stop"}"#,
    "\n\n",
);

/// Feed a transcript through the framer and accumulator as ONE chunk.
fn accumulate(transcript: &str) -> Result<StreamedMessage, StreamError> {
    accumulate_chunks(&[transcript.as_bytes()])
}

/// Feed an arbitrary chunking of a transcript, exactly as a socket would
/// deliver it — arbitrary splits, no respect for line or character boundaries.
fn accumulate_chunks(chunks: &[&[u8]]) -> Result<StreamedMessage, StreamError> {
    let mut decoder = SseDecoder::new();
    let mut accumulator = MessageAccumulator::new();
    for chunk in chunks {
        for payload in decoder.push_bytes(chunk)? {
            if accumulator.push(&payload)? == Progress::Done {
                return accumulator.finish();
            }
        }
    }
    accumulator.finish()
}

/// Swap the `stop_reason` in the happy transcript for another value.
fn with_stop_reason(reason: &str) -> String {
    HAPPY_TRANSCRIPT.replace(
        r#""stop_reason":"end_turn""#,
        &format!(r#""stop_reason":"{reason}""#),
    )
}

#[test]
fn a_streamed_response_rebuilds_the_text_the_usage_and_the_stop_reason() {
    let message = accumulate(HAPPY_TRANSCRIPT).expect("a well-formed transcript must accumulate");

    // The two text deltas concatenate WITHIN a block — no separator. Only
    // separate blocks join with a newline.
    assert_eq!(message.text, r#"{"entities":[]}"#);
    // Input tokens come from `message_start`, output tokens from the FINAL
    // `message_delta` — not from the placeholder `1` on `message_start`.
    assert_eq!(message.input_tokens, Some(41255));
    assert_eq!(message.output_tokens, Some(18));
    assert_eq!(message.stop_reason, "end_turn");
    assert_eq!(message.message_id.as_deref(), Some("msg_01Test"));
}

#[test]
fn a_response_cut_off_at_the_ceiling_reports_stop_reason_max_tokens() {
    // The incident shape, and the one the truncation gate exists for. If the
    // streaming transport ever stopped carrying this value out of
    // `message_delta`, census R-3 would silently reopen: `repair_json` would
    // close the truncated array and a cut-off extraction would store as a
    // complete one. The end-to-end consequence — that this classifies TERMINAL
    // — is pinned in `workflow_steps::llm_extract_tests`.
    let message = accumulate(&with_stop_reason("max_tokens"))
        .expect("a truncated response is still a well-formed stream");
    assert_eq!(
        message.stop_reason,
        crate::pipeline::truncation::STOP_REASON_MAX_TOKENS
    );
}

#[test]
fn arbitrary_socket_chunking_including_a_split_multibyte_character_is_reassembled() {
    // A section sign is two bytes in UTF-8. Splitting the stream between them is
    // what a `String` buffer could not survive, and what a lossy decode would
    // turn into a replacement character that fails grounding much later.
    let transcript = HAPPY_TRANSCRIPT.replace(r#"[]}"#, r#"[]} §"#);
    let bytes = transcript.as_bytes();

    // Byte-at-a-time is the most hostile chunking there is: every line, every
    // event, and every character boundary is crossed by a chunk boundary.
    let per_byte: Vec<&[u8]> = bytes.chunks(1).collect();
    let one_shot = accumulate(&transcript).expect("one-shot must accumulate");
    let dribbled = accumulate_chunks(&per_byte).expect("byte-at-a-time must accumulate");
    assert_eq!(one_shot, dribbled, "chunking must not change the message");
}

#[test]
fn a_thinking_block_is_skipped_and_separate_text_blocks_join_with_a_newline() {
    let transcript = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_02","usage":{"input_tokens":10,"output_tokens":1}}}"#,
        "\n\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}"#,
        "\n\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"pondering"}}"#,
        "\n\n",
        r#"data: {"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#,
        "\n\n",
        r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"first"}}"#,
        "\n\n",
        r#"data: {"type":"content_block_start","index":2,"content_block":{"type":"text","text":""}}"#,
        "\n\n",
        r#"data: {"type":"content_block_delta","index":2,"delta":{"type":"text_delta","text":"second"}}"#,
        "\n\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":9}}"#,
        "\n\n",
        r#"data: {"type":"message_stop"}"#,
        "\n\n",
    );
    let message = accumulate(transcript).expect("mixed block types must accumulate");
    // "pondering" must NOT appear: the non-streaming path collected only text
    // blocks, and the transports must not disagree about what the answer is.
    assert_eq!(message.text, "first\nsecond");
}

#[test]
fn an_error_event_mid_stream_fails_the_call() {
    let transcript = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_03","usage":{"input_tokens":10}}}"#,
        "\n\n",
        r#"data: {"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#,
        "\n\n",
    );
    match accumulate(transcript) {
        Err(StreamError::ProviderEvent { kind, message }) => {
            assert_eq!(kind, "overloaded_error");
            assert_eq!(message, "Overloaded");
        }
        other => panic!("expected ProviderEvent, got {other:?}"),
    }
}

#[test]
fn a_stream_that_ends_early_is_incomplete_and_never_a_partial_answer() {
    // The dropped-connection shape. The bytes received so far ARE parseable
    // prose, and returning them would be the census R-3 defect wearing a
    // different hat: `repair_json` would close the partial JSON and store it as
    // a complete extraction.
    let truncated_transcript = HAPPY_TRANSCRIPT
        .split("event: message_delta")
        .next()
        .expect("split always yields a first element");
    match accumulate(truncated_transcript) {
        Err(StreamError::Incomplete { events_seen }) => {
            assert!(
                events_seen > 0,
                "the count must report the events that DID arrive, got {events_seen}"
            );
        }
        other => panic!("expected Incomplete, got {other:?}"),
    }
}

#[test]
fn a_message_stop_with_no_stop_reason_fails_rather_than_disarming_the_gate() {
    let transcript = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_04","usage":{"input_tokens":10}}}"#,
        "\n\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
        "\n\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hi"}}"#,
        "\n\n",
        r#"data: {"type":"message_stop"}"#,
        "\n\n",
    );
    assert!(
        matches!(accumulate(transcript), Err(StreamError::MissingStopReason)),
        "a stream with no stop_reason must fail, not report None — reporting None \
         is indistinguishable from a provider that never reports the field, and it \
         turns the truncation gate off for every call"
    );
}

#[test]
fn an_event_type_anthropic_adds_later_is_ignored_rather_than_fatal() {
    let transcript = HAPPY_TRANSCRIPT.replace(
        r#"data: {"type":"ping"}"#,
        r#"data: {"type":"some_future_event","payload":{"anything":1}}"#,
    );
    let message = accumulate(&transcript)
        .expect("an unknown event type must not fail an otherwise good stream");
    assert_eq!(message.stop_reason, "end_turn");
}

#[test]
fn a_payload_that_is_not_json_names_itself_in_the_failure() {
    let transcript = "data: this is not json\n\n";
    match accumulate(transcript) {
        Err(StreamError::MalformedEvent { preview, .. }) => {
            assert!(
                preview.contains("this is not json"),
                "the failure must quote the payload; got: {preview}"
            );
        }
        other => panic!("expected MalformedEvent, got {other:?}"),
    }
}

#[test]
fn crlf_framing_and_multi_line_data_fields_are_handled() {
    // SSE permits several `data:` lines per event, joined with newlines, and a
    // `\r\n` stream is legal. A stray `\r` left on a payload would fail the JSON
    // parse with a message that gave no hint that framing was the cause.
    let mut decoder = SseDecoder::new();
    let payloads = decoder
        .push_bytes(b"data: {\"type\":\r\ndata: \"ping\"}\r\n\r\n")
        .expect("valid UTF-8 framing must decode");
    assert_eq!(payloads, vec!["{\"type\":\n\"ping\"}".to_string()]);
}

#[test]
fn a_partial_line_yields_nothing_until_its_newline_arrives() {
    // The property the idle clock depends on: bytes arriving is not the same as
    // an event arriving. See `anthropic_transport`.
    let mut decoder = SseDecoder::new();
    assert!(decoder
        .push_bytes(br#"data: {"type":"ping"#)
        .expect("valid UTF-8")
        .is_empty());
    assert_eq!(
        decoder.push_bytes(b"\"}\n\n").expect("valid UTF-8"),
        vec![r#"{"type":"ping"}"#.to_string()]
    );
}
