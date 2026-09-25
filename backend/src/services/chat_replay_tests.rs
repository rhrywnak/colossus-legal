//! Tests for `chat_replay`: what a replayed thread SENDS, from rows built in
//! memory (no database). The stale citation below carries the real values of
//! thread 16a656b7's first reply (CC_TASK_CITATION_REPLAY_CHECK_v1 §2).

use chrono::Utc;
use serde_json::{json, Value};

use super::*;
use crate::domain::chat_params::QuestionChatParams;
use crate::services::chat_case_prefix::build_request;

/// One stored row, as `list_messages` returns it.
fn row(seq: i32, role: &str, content: Value, failure: Option<&str>) -> MessageRecord {
    MessageRecord {
        seq,
        role: role.to_string(),
        content,
        rendered_text: None,
        citations: None,
        model: None,
        stop_reason: None,
        failure: failure.map(str::to_string),
        created_at: Utc::now(),
    }
}

fn user(seq: i32, text: &str) -> MessageRecord {
    row(seq, "user", json!([{"type": "text", "text": text}]), None)
}

/// A reply citing position 7 at a range the document now at position 7 is too
/// short for — the exact shape the provider refused with a 400.
fn stale_reply(seq: i32) -> MessageRecord {
    row(
        seq,
        "assistant",
        json!([
            {"type": "thinking", "thinking": "", "signature": "sig-abc"},
            {"type": "text", "text": "The court held "},
            {"type": "text", "text": "that the appeal failed",
             "citations": [{"type": "char_location", "cited_text": "the appeal failed",
                "document_index": 7, "document_title": "COURT OF APPEALS RULLING 01 12 2012",
                "start_char_index": 38268, "end_char_index": 38285}]},
            {"type": "text", "text": "."}
        ]),
        None,
    )
}

/// A content-free failure marker, as `store_failure` writes it.
fn failure(seq: i32) -> MessageRecord {
    row(seq, "assistant", json!([]), Some("failed"))
}

/// True when any block, at any depth, still has a `citations` key.
fn has_citations(v: &Value) -> bool {
    match v {
        Value::Object(m) => m.contains_key("citations") || m.values().any(has_citations),
        Value::Array(items) => items.iter().any(has_citations),
        _ => false,
    }
}

/// Journey 1: an out-of-range citation is not sent; the reply's text is.
#[test]
fn a_stale_citation_is_not_sent_and_the_text_is_unchanged() {
    let stored = stale_reply(2);
    let before: Vec<Value> = stored.content.as_array().cloned().unwrap_or_default();
    let sent = replayable(Uuid::nil(), vec![user(1, "Q"), stored]);
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[1].role, Role::Assistant);
    assert!(!has_citations(&json!(sent[1].content)));
    let texts = |blocks: &[Value]| -> Vec<Value> {
        blocks
            .iter()
            .map(|b| b.get("text").cloned().unwrap_or(Value::Null))
            .collect()
    };
    assert_eq!(texts(&sent[1].content), texts(&before));
    assert_eq!(
        sent[1].content[2],
        json!({"type": "text", "text": "that the appeal failed"})
    );
}

/// Signed blocks go out byte-identical, even if one (hypothetically) carried a
/// `citations` key: the provider checks them on replay.
#[test]
fn thinking_blocks_are_sent_byte_identical() {
    let thinking = json!({"type": "thinking", "thinking": "t", "signature": "s1",
        "citations": ["kept"]});
    let redacted = json!({"type": "redacted_thinking", "data": "opaque"});
    let stored = row(
        2,
        "assistant",
        json!([thinking.clone(), redacted.clone(), {"type": "text", "text": "x"}]),
        None,
    );
    let sent = replayable(Uuid::nil(), vec![user(1, "Q"), stored]);
    assert_eq!(sent[1].content[0], thinking);
    assert_eq!(sent[1].content[1], redacted);
}

/// Blocks without positions pass through untouched.
#[test]
fn tool_use_tool_result_and_compaction_pass_through() {
    let tool_use = json!({"type": "tool_use", "id": "tu_1", "name": "get_pages",
        "input": {"id": "doc-1", "first_page": 2}});
    let compaction = json!({"type": "compaction", "content": "summary of earlier turns"});
    let tool_result = json!({"type": "tool_result", "tool_use_id": "tu_1",
        "content": "page text", "is_error": false});
    let sent = replayable(
        Uuid::nil(),
        vec![
            user(1, "Q"),
            row(
                2,
                "assistant",
                json!([compaction.clone(), tool_use.clone()]),
                None,
            ),
            row(3, "user", json!([tool_result.clone()]), None),
        ],
    );
    assert_eq!(sent[1].content, vec![compaction, tool_use]);
    assert_eq!(sent[2].content, vec![tool_result]);
}

/// User rows are sent exactly as stored — the strip applies to replies only.
#[test]
fn user_messages_are_sent_as_stored() {
    let content = json!([{"type": "text", "text": "Q", "citations": ["stray"]}]);
    let sent = replayable(Uuid::nil(), vec![row(1, "user", content.clone(), None)]);
    assert_eq!(sent[0].role, Role::User);
    assert_eq!(json!(sent[0].content), content);
}

/// Journey 2: 16a656b7's shape — a cited reply, then three questions each
/// followed by a failure marker, then the new question — builds a valid request:
/// no failure markers, no citations, her four questions in order.
#[test]
fn the_16a656b7_shape_is_a_valid_request() {
    let rows = vec![
        user(1, "first"),
        stale_reply(2),
        user(69, "try 1"),
        failure(70),
        user(71, "try 2"),
        failure(72),
        user(73, "try 3"),
        failure(74),
        user(75, "try 4"),
    ];
    let history = replayable(Uuid::nil(), rows);
    let roles: Vec<Role> = history.iter().map(|m| m.role).collect();
    use Role::{Assistant as A, User as U};
    assert_eq!(roles, vec![U, A, U, U, U, U]);
    assert!(history.iter().all(|m| !m.content.is_empty()));

    let chat = QuestionChatParams::for_test();
    let request = build_request(
        &chat,
        "prompt".into(),
        "narrative".into(),
        "context".into(),
        &[],
        history,
    );
    let body = colossus_chat::request::build_body(&request);
    let body = match body {
        Ok(b) => b,
        Err(e) => panic!("the replayed history must build a request: {e}"),
    };
    let messages = body["messages"].as_array().cloned().unwrap_or_default();
    // messages[0] is the documents + context message; the history follows it.
    assert_eq!(messages.len(), 7);
    assert!(!has_citations(&json!(messages[1..])));
    assert_eq!(messages[6]["content"][0]["text"], "try 4");
}

/// A turn that ended in a refusal keeps its tool round; only the failed final
/// reply is dropped, so no tool_result is left without its tool_use.
#[test]
fn a_turn_ended_by_refusal_keeps_its_tool_round() {
    let tool_use = json!({"type": "tool_use", "id": "tu_9", "name": "list_documents",
        "input": {}});
    let sent = replayable(
        Uuid::nil(),
        vec![
            user(1, "Q"),
            row(2, "assistant", json!([tool_use.clone()]), None),
            row(
                3,
                "user",
                json!([{"type": "tool_result", "tool_use_id": "tu_9",
            "content": "list", "is_error": false}]),
                None,
            ),
            row(
                4,
                "assistant",
                json!([{"type": "text", "text": "no"}]),
                Some("refused"),
            ),
            user(5, "Q again"),
        ],
    );
    let roles: Vec<Role> = sent.iter().map(|m| m.role).collect();
    assert_eq!(
        roles,
        vec![Role::User, Role::Assistant, Role::User, Role::User]
    );
    assert_eq!(sent[1].content, vec![tool_use]);
}

/// A store defect (content that is not an array) is sent as an empty message
/// and does not take the rest of the thread with it.
#[test]
fn a_non_array_content_row_is_sent_empty_and_the_rest_survive() {
    let sent = replayable(
        Uuid::nil(),
        vec![row(1, "user", json!({"oops": true}), None), user(2, "Q")],
    );
    assert_eq!(sent.len(), 2);
    assert!(sent[0].content.is_empty());
    assert_eq!(sent[1].content[0]["text"], "Q");
}

/// A thread that cannot be read is a named `Store` error that carries the
/// operation and the question, never an empty history sent as if it were fine.
/// The pool aims at a dead port (the `scenario_candidate_ordinals` precedent),
/// so the read fails fast without a database.
#[tokio::test]
async fn an_unreadable_thread_is_a_named_store_error() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .connect_lazy("postgres://127.0.0.1:1/nodb")
        .expect("connect_lazy builds a pool without connecting");
    let question = Uuid::from_u128(7);
    match replay_history(&pool, question, Uuid::nil()).await {
        Err(ChatRunError::Store {
            operation,
            question_id,
            ..
        }) => {
            assert_eq!(operation, "list_messages");
            assert_eq!(question_id, question);
        }
        other => panic!("expected a Store error, got {other:?}"),
    }
}
