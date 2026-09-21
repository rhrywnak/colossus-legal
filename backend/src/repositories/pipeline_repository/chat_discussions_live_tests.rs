//! Live-database proofs for `chat_discussions` (CC_TASK_CHAT_ENGINE_v1).
//!
//! `#[ignore]`d, run by hand against a SCRATCH copy of the pipeline schema with
//! this task's migration applied — never `colossus_legal_v2`.

use serde_json::json;

use super::super::war_room_status::live_tests::{
    cleanup, pipeline_pool, question, scenario, TestResult,
};
use super::*;

/// Threads have no foreign key to their question (ruling D5), so the scenario's
/// cascade does not reach them — they are deleted first, by anchor.
async fn cleanup_chat(pool: &PgPool, s: Uuid, tag: &str, q: Uuid) -> TestResult<()> {
    sqlx::query("DELETE FROM discussions WHERE anchor_id = $1")
        .bind(q.to_string())
        .execute(pool)
        .await?;
    cleanup(pool, s, tag).await
}

fn user(text: &str) -> NewMessage {
    NewMessage {
        role: "user",
        content: json!([{"type": "text", "text": text}]),
        rendered_text: Some(text.to_string()),
        ..NewMessage::default()
    }
}

fn assistant(text: &str) -> NewMessage {
    NewMessage {
        role: "assistant",
        content: json!([{"type": "text", "text": text}]),
        rendered_text: Some(text.to_string()),
        model: Some("claude-opus-5".into()),
        stop_reason: Some("end_turn".into()),
        ..NewMessage::default()
    }
}

/// Appends are numbered 1, 2, 3 and read back in order, verbatim.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn append_is_monotonic_and_verbatim() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "chat_append").await?;
    let q = question(&pool, s, "george", 1).await?;
    let d = get_or_create_discussion(&pool, ANCHOR_QUESTION, &q.to_string(), "docmarie").await?;
    assert_eq!(append_message(&pool, d.id, &user("why?")).await?, 1);
    assert_eq!(append_message(&pool, d.id, &assistant("because")).await?, 2);
    assert_eq!(
        append_message(
            &pool,
            d.id,
            &NewMessage {
                rendered_text: None,
                content: json!([{"type": "tool_result", "tool_use_id": "t", "content": "x"}]),
                ..user("")
            }
        )
        .await?,
        3
    );
    let msgs = list_messages(&pool, d.id).await?;
    assert_eq!(
        msgs.iter().map(|m| m.seq).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(
        msgs[1].content,
        json!([{"type": "text", "text": "because"}])
    );
    assert_eq!(assistant_reply_count(&pool, d.id).await?, 1);
    cleanup_chat(&pool, s, "chat_append", q).await
}

/// One thread per person per question: a second create returns the same row.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn one_thread_per_user_per_anchor() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "chat_one").await?;
    let q = question(&pool, s, "george", 1).await?;
    let a = get_or_create_discussion(&pool, ANCHOR_QUESTION, &q.to_string(), "docmarie").await?;
    let b = get_or_create_discussion(&pool, ANCHOR_QUESTION, &q.to_string(), "docmarie").await?;
    let c = get_or_create_discussion(&pool, ANCHOR_QUESTION, &q.to_string(), "cpenzien").await?;
    assert_eq!(a.id, b.id);
    assert_ne!(a.id, c.id);
    cleanup_chat(&pool, s, "chat_one", q).await
}

/// The switcher's counts: visible messages, unread past the viewer's mark, preview.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn thread_list_counts_unread_and_mark_read() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "chat_list").await?;
    let q = question(&pool, s, "george", 1).await?;
    let anchor = q.to_string();
    let chuck = get_or_create_discussion(&pool, ANCHOR_QUESTION, &anchor, "cpenzien").await?;
    append_message(&pool, chuck.id, &user("cleanest exhibit?")).await?;
    append_message(&pool, chuck.id, &assistant("the 2016 response")).await?;

    let rows = list_threads(&pool, ANCHOR_QUESTION, &anchor, "docmarie").await?;
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].visible_count, rows[0].unread), (2, 2));
    assert_eq!(rows[0].last_text.as_deref(), Some("the 2016 response"));

    mark_read(&pool, chuck.id, "docmarie", 2).await?;
    mark_read(&pool, chuck.id, "docmarie", 1).await?; // never moves back
    let rows = list_threads(&pool, ANCHOR_QUESTION, &anchor, "docmarie").await?;
    assert_eq!(rows[0].unread, 0);
    assert_eq!(rows[0].seen_seq, 2);
    cleanup_chat(&pool, s, "chat_list", q).await
}
