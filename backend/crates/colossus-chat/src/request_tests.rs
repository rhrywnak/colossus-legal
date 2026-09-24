//! Request-body tests: the shape the API receives, asserted without sending it.

use super::*;

fn user(text: &str) -> Message {
    Message {
        role: Role::User,
        content: vec![json!({"type":"text","text":text})],
    }
}

fn req() -> ChatRequest {
    ChatRequest {
        model: "m".into(),
        max_tokens: 100,
        system: vec!["prompt".into(), "narrative".into()],
        documents: vec![
            ChatDocument {
                title: "A — 2016".into(),
                context: Some("dated".into()),
                text: "Alpha.".into(),
            },
            ChatDocument {
                title: "B".into(),
                context: None,
                text: "Beta.".into(),
            },
        ],
        context: "the question".into(),
        history: vec![user("first"), user("second")],
        tools: vec![],
        effort: Some("high".into()),
        adaptive_thinking: true,
        compaction_trigger_tokens: None,
        cache_ttl: CacheTtl::OneHour,
    }
}

#[test]
fn every_document_carries_citations_and_plain_text_source() {
    let body = build_body(&req()).unwrap();
    let first = &body["messages"][0]["content"];
    for i in 0..2 {
        assert_eq!(first[i]["type"], "document");
        assert_eq!(first[i]["citations"], json!({"enabled": true}));
        assert_eq!(first[i]["source"]["type"], "text");
    }
    assert_eq!(first[0]["context"], "dated");
    assert!(first[1].get("context").is_none());
    assert_eq!(first[2], json!({"type":"text","text":"the question"}));
}

#[test]
fn breakpoints_sit_on_system_last_document_and_last_turn() {
    let body = build_body(&req()).unwrap();
    assert!(body["system"][0].get("cache_control").is_none());
    assert_eq!(
        body["system"][1]["cache_control"],
        json!({"type":"ephemeral","ttl":"1h"})
    );
    assert!(body["messages"][0]["content"][0]
        .get("cache_control")
        .is_none());
    assert!(body["messages"][0]["content"][1]
        .get("cache_control")
        .is_some());
    // messages[1] = "first" (no marker), messages[2] = "second" (marker).
    assert!(body["messages"][1]["content"][0]
        .get("cache_control")
        .is_none());
    assert!(body["messages"][2]["content"][0]
        .get("cache_control")
        .is_some());
    assert_eq!(count_breakpoints(&body), 3);
}

#[test]
fn thinking_effort_and_compaction_follow_configuration() {
    let mut r = req();
    r.compaction_trigger_tokens = Some(600_000);
    let body = build_body(&r).unwrap();
    assert_eq!(body["thinking"], json!({"type":"adaptive"}));
    assert_eq!(body["output_config"], json!({"effort":"high"}));
    assert_eq!(
        body["context_management"]["edits"][0]["type"],
        "compact_20260112"
    );
    assert_eq!(
        body["context_management"]["edits"][0]["trigger"]["value"],
        600_000
    );
    let mut r = req();
    r.effort = None;
    r.adaptive_thinking = false;
    let body = build_body(&r).unwrap();
    assert!(body.get("thinking").is_none() && body.get("output_config").is_none());
    assert!(body.get("context_management").is_none());
}

#[test]
fn a_conversation_must_end_with_the_user() {
    let mut r = req();
    r.history.push(Message {
        role: Role::Assistant,
        content: vec![json!({"type":"text","text":"x"})],
    });
    assert_eq!(
        build_body(&r),
        Err(RequestError::NotUserLast("an assistant message"))
    );
    r.history.clear();
    assert_eq!(build_body(&r), Err(RequestError::NotUserLast("nothing")));
}

#[test]
fn an_empty_document_is_refused() {
    let mut r = req();
    r.documents[1].text = "  ".into();
    assert_eq!(build_body(&r), Err(RequestError::EmptyDocument("B".into())));
}

#[test]
fn a_fifth_breakpoint_is_refused() {
    let mut r = req();
    // A caller-supplied history block already carrying markers pushes the total over.
    for _ in 0..2 {
        r.history.insert(
            0,
            Message {
                role: Role::User,
                content: vec![
                    json!({"type":"text","text":"x","cache_control":{"type":"ephemeral"}}),
                ],
            },
        );
    }
    assert_eq!(build_body(&r), Err(RequestError::TooManyBreakpoints(5)));
}

#[test]
fn ttl_parses_only_the_two_wire_spellings() {
    assert_eq!(CacheTtl::parse("1h"), Ok(CacheTtl::OneHour));
    assert_eq!(CacheTtl::parse(" 5m "), Ok(CacheTtl::FiveMinutes));
    assert_eq!(
        CacheTtl::parse("2h"),
        Err(RequestError::BadTtl("2h".into()))
    );
}

#[test]
fn the_count_body_is_the_send_body_minus_what_the_endpoint_rejects() {
    let mut r = req();
    r.compaction_trigger_tokens = Some(700_000);
    let sent = build_body(&r).unwrap();
    let counted = build_count_body(&r).unwrap();

    // The four generation keys are present on the real body and gone from the count.
    for key in [
        "stream",
        "max_tokens",
        "output_config",
        "context_management",
    ] {
        assert!(sent.get(key).is_some(), "`{key}` belongs on the sent body");
        assert!(
            counted.get(key).is_none(),
            "count_tokens rejects `{key}`, so it must be stripped"
        );
    }
    // Everything that decides the SIZE is byte-identical — which is the only
    // reason the count describes what will be sent.
    for key in ["model", "system", "messages", "thinking"] {
        assert_eq!(sent.get(key), counted.get(key), "`{key}` must not differ");
    }
}

#[test]
fn the_count_body_refuses_the_same_requests_build_body_refuses() {
    let mut r = req();
    r.history = vec![Message {
        role: Role::Assistant,
        content: vec![json!({"type":"text","text":"x"})],
    }];
    assert_eq!(
        build_count_body(&r),
        Err(RequestError::NotUserLast("an assistant message"))
    );
}

/// A request with a tool and compaction on, so the pre-warm tests can see that
/// every prefix-shaping key survives the derivation.
fn full_req() -> ChatRequest {
    ChatRequest {
        tools: vec![ToolSpec {
            name: "list_documents".into(),
            description: "List them.".into(),
            input_schema: json!({"type": "object"}),
        }],
        compaction_trigger_tokens: Some(700_000),
        ..req()
    }
}

/// The pre-warm is only worth sending if the provider sees the SAME prefix a
/// real turn sends: tools, system, model, the documents, and the thinking /
/// effort / compaction settings rendered into the prompt.
#[test]
fn the_prewarm_body_shares_every_prefix_byte_with_the_real_body() {
    let real = build_body(&full_req()).unwrap();
    let warm = build_prewarm_body(&full_req()).unwrap();
    for key in [
        "model",
        "tools",
        "system",
        "thinking",
        "output_config",
        "context_management",
    ] {
        assert_eq!(warm[key], real[key], "`{key}` differs");
    }
    // messages[0] is exactly the real turn's documents, breakpoint included.
    let real_docs = &real["messages"][0]["content"].as_array().unwrap()[..2];
    assert_eq!(
        warm["messages"][0]["content"].as_array().unwrap(),
        real_docs
    );
    assert_eq!(warm["messages"][0]["role"], "user");
}

/// `max_tokens: 0` is the documented pre-warm, and the provider refuses it
/// together with `stream`.
#[test]
fn the_prewarm_body_asks_for_nothing_and_does_not_stream() {
    let warm = build_prewarm_body(&full_req()).unwrap();
    assert_eq!(warm["max_tokens"], 0);
    assert!(warm.get("stream").is_none());
}

/// Nothing after the document breakpoint is sent: no per-question context, no
/// conversation — so there is no tail for the pre-warm to WRITE.
#[test]
fn the_prewarm_body_keeps_only_the_documents() {
    let warm = build_prewarm_body(&full_req()).unwrap();
    let messages = warm["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    let content = messages[0]["content"].as_array().unwrap();
    assert_eq!(content.len(), 2);
    assert!(content.iter().all(|b| b["type"] == "document"));
}

/// Two breakpoints: the system prompt and the last document. The conversation's
/// breakpoint is gone with the conversation.
#[test]
fn the_prewarm_body_places_exactly_two_breakpoints() {
    let warm = build_prewarm_body(&full_req()).unwrap();
    assert_eq!(count_breakpoints(&warm), 2);
    assert!(warm["system"][1].get("cache_control").is_some());
    assert!(warm["messages"][0]["content"][1]
        .get("cache_control")
        .is_some());
}

/// With no documents there is nothing the next turn could read back, so the
/// pre-warm refuses by name rather than paying for a system-prompt-only write.
#[test]
fn a_prewarm_with_no_documents_is_refused() {
    let mut r = full_req();
    r.documents.clear();
    assert_eq!(build_prewarm_body(&r), Err(RequestError::NothingToWarm));
}
