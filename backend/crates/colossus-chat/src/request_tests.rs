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
