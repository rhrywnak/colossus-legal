//! Accumulator tests against recorded-shape transcripts. No socket, no tokens.

use super::*;
use serde_json::json;

/// Fold a list of event objects; return the finished message.
fn fold(events: &[Value]) -> Result<AssistantMessage, ChatStreamError> {
    let mut acc = ChatAccumulator::new(200);
    for e in events {
        if acc.push(&e.to_string())? == Progress::Done {
            break;
        }
    }
    acc.finish()
}

fn start(usage: Value) -> Value {
    json!({"type":"message_start","message":{"id":"msg_1","usage":usage}})
}

fn stop(reason: &str) -> [Value; 2] {
    [
        json!({"type":"message_delta","delta":{"stop_reason":reason},"usage":{"output_tokens":42}}),
        json!({"type":"message_stop"}),
    ]
}

#[test]
fn citations_delta_attaches_to_its_text_block() {
    let citation = json!({"type":"char_location","cited_text":"The letter is dated November 5, 2009.",
        "document_index":0,"document_title":"Letter","start_char_index":0,"end_char_index":37});
    let mut events = vec![
        start(json!({"input_tokens":12,"cache_read_input_tokens":170000})),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Your letter "}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"content_block_start","index":1,"content_block":{"type":"text","text":"","citations":[]}}),
        json!({"type":"content_block_delta","index":1,"delta":{"type":"citations_delta","citation":citation}}),
        json!({"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"is dated November 5"}}),
        json!({"type":"content_block_stop","index":1}),
    ];
    events.extend(stop("end_turn"));
    let m = fold(&events).unwrap();
    assert_eq!(m.content.len(), 2);
    assert_eq!(m.content[1]["citations"][0], citation);
    assert_eq!(m.text(), "Your letter is dated November 5");
    assert_eq!(m.usage.cache_read_input_tokens, Some(170000));
    assert_eq!(m.usage.output_tokens, Some(42));
}

#[test]
fn thinking_signature_and_compaction_are_kept_verbatim() {
    let mut events = vec![
        start(json!({"input_tokens":1})),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"compaction"}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"compaction_delta","content":"Summary."}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"content_block_start","index":1,"content_block":{"type":"thinking","thinking":"","signature":""}}),
        json!({"type":"content_block_delta","index":1,"delta":{"type":"thinking_delta","thinking":"hm"}}),
        json!({"type":"content_block_delta","index":1,"delta":{"type":"signature_delta","signature":"SIG=="}}),
        json!({"type":"content_block_stop","index":1}),
        json!({"type":"content_block_start","index":2,"content_block":{"type":"redacted_thinking","data":"opaque"}}),
        json!({"type":"content_block_stop","index":2}),
    ];
    events.extend(stop("end_turn"));
    let m = fold(&events).unwrap();
    assert_eq!(
        m.content[0],
        json!({"type":"compaction","content":"Summary."})
    );
    assert_eq!(
        m.content[1],
        json!({"type":"thinking","thinking":"hm","signature":"SIG=="})
    );
    assert_eq!(
        m.content[2],
        json!({"type":"redacted_thinking","data":"opaque"})
    );
    assert_eq!(m.text(), "");
}

#[test]
fn tool_input_streams_and_parses_at_block_stop() {
    let mut events = vec![
        start(json!({})),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"tu_1","name":"get_document","input":{}}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"id\":"}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"\"d1\"}"}}),
        json!({"type":"content_block_stop","index":0}),
    ];
    events.extend(stop("tool_use"));
    let m = fold(&events).unwrap();
    assert_eq!(m.content[0]["input"], json!({"id":"d1"}));
    assert_eq!(m.stop_reason, "tool_use");
}

#[test]
fn a_tool_call_with_no_streamed_input_is_an_empty_object() {
    let mut events = vec![
        start(json!({})),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"tu","name":"list_documents","input":{}}}),
        json!({"type":"content_block_stop","index":0}),
    ];
    events.extend(stop("tool_use"));
    assert_eq!(fold(&events).unwrap().content[0]["input"], json!({}));
}

#[test]
fn malformed_tool_input_is_a_named_error() {
    let mut events = vec![
        start(json!({})),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"tu","name":"t","input":{}}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"id\""}}),
        json!({"type":"content_block_stop","index":0}),
    ];
    events.extend(stop("tool_use"));
    assert!(matches!(
        fold(&events),
        Err(ChatStreamError::ToolInput { .. })
    ));
}

#[test]
fn an_unknown_delta_is_a_named_error_not_dropped() {
    let events = vec![
        start(json!({})),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"hologram_delta","x":1}}),
    ];
    match fold(&events) {
        Err(ChatStreamError::UnknownDelta { kind, index }) => {
            assert_eq!(kind, "hologram_delta");
            assert_eq!(index, 0);
        }
        other => panic!("expected UnknownDelta, got {other:?}"),
    }
}

#[test]
fn an_unknown_block_is_a_named_error() {
    let events = vec![
        start(json!({})),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"server_tool_use"}}),
    ];
    assert!(matches!(
        fold(&events),
        Err(ChatStreamError::UnknownBlock { .. })
    ));
}

#[test]
fn ping_and_unknown_events_are_ignored() {
    let mut events = vec![
        start(json!({})),
        json!({"type":"ping"}),
        json!({"type":"new_envelope"}),
    ];
    events.extend(stop("end_turn"));
    assert_eq!(fold(&events).unwrap().stop_reason, "end_turn");
}

#[test]
fn a_stream_without_message_stop_is_incomplete() {
    let events = vec![start(json!({}))];
    assert!(matches!(
        fold(&events),
        Err(ChatStreamError::Incomplete { events_seen: 1 })
    ));
}

#[test]
fn a_stream_without_stop_reason_is_refused() {
    let events = vec![start(json!({})), json!({"type":"message_stop"})];
    assert!(matches!(
        fold(&events),
        Err(ChatStreamError::MissingStopReason)
    ));
}

#[test]
fn an_error_event_names_the_provider_error() {
    let events = vec![
        start(json!({})),
        json!({"type":"error","error":{"type":"overloaded_error","message":"busy"}}),
    ];
    match fold(&events) {
        Err(ChatStreamError::ProviderEvent { kind, message }) => {
            assert_eq!(
                (kind.as_str(), message.as_str()),
                ("overloaded_error", "busy")
            );
        }
        other => panic!("expected ProviderEvent, got {other:?}"),
    }
}

#[test]
fn take_text_yields_only_fresh_prose() {
    let mut acc = ChatAccumulator::new(200);
    acc.push(&start(json!({})).to_string()).unwrap();
    acc.push(
        &json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}})
            .to_string(),
    )
    .unwrap();
    acc.push(
        &json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"a"}})
            .to_string(),
    )
    .unwrap();
    assert_eq!(acc.take_text(), "a");
    assert_eq!(acc.take_text(), "");
}
