//! Storage for the discussion threads (CC_TASK_CHAT_ENGINE_v1), plus the one read
//! of the document corpus the chat is given.
//!
//! Three tables from migration `20260921140958`: `discussions` (one per anchor and
//! user), `discussion_messages` (append-only, verbatim API content) and
//! `discussion_reads` (how far each viewer has read).
//!
//! ## Domain note: append-only, enforced by having no UPDATE
//!
//! There is no function here that edits a message. The provider checks thinking
//! signatures against the history they were produced in, so an edited turn would
//! silently invalidate every later one. The absence of an update path is the rule.

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

/// CONST: the one anchor type this build writes (the column's CHECK vocabulary).
pub const ANCHOR_QUESTION: &str = "question";

/// A thread's header row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DiscussionRecord {
    pub id: Uuid,
    pub username: String,
    pub created_at: DateTime<Utc>,
}

/// One thread on an anchor, summarized for the switcher and for the viewer.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ThreadSummary {
    pub id: Uuid,
    pub username: String,
    pub created_at: DateTime<Utc>,
    /// Messages a person sees (user and assistant turns with text).
    pub visible_count: i64,
    /// The highest seq in the thread (0 when empty).
    pub last_seq: i32,
    /// The viewer's read position (0 when they never opened it).
    pub seen_seq: i32,
    /// Visible messages past the viewer's read position.
    pub unread: i64,
    /// The last visible message's text, for the one-line preview.
    pub last_text: Option<String>,
}

/// One stored message.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MessageRecord {
    pub seq: i32,
    pub role: String,
    pub content: Value,
    pub rendered_text: Option<String>,
    pub citations: Option<Value>,
    pub model: Option<String>,
    pub stop_reason: Option<String>,
    pub failure: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A message about to be stored. Token counts are `None` when not reported.
#[derive(Debug, Clone, Default)]
pub struct NewMessage {
    pub role: &'static str,
    pub content: Value,
    pub rendered_text: Option<String>,
    pub citations: Option<Value>,
    pub model: Option<String>,
    pub stop_reason: Option<String>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub cache_creation_tokens: Option<i32>,
    pub cache_read_tokens: Option<i32>,
    pub ms: Option<i32>,
    pub failure: Option<String>,
}

/// One stored document with its text, page by page.
#[derive(Debug, Clone, PartialEq)]
pub struct CorpusDocument {
    pub id: String,
    pub title: String,
    pub document_date: Option<NaiveDate>,
    /// `(page_number, text)`, in page order.
    pub pages: Vec<(i32, String)>,
}

/// The owner's thread on an anchor, if one exists.
///
/// # Errors
/// A database failure.
pub async fn find_discussion(
    pool: &PgPool,
    anchor_type: &str,
    anchor_id: &str,
    username: &str,
) -> Result<Option<DiscussionRecord>, PipelineRepoError> {
    sqlx::query_as::<_, DiscussionRecord>(
        "SELECT id, username, created_at FROM discussions \
         WHERE anchor_type = $1 AND anchor_id = $2 AND username = $3",
    )
    .bind(anchor_type)
    .bind(anchor_id)
    .bind(username)
    .fetch_optional(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// The owner's thread on an anchor, created on first use.
///
/// `ON CONFLICT DO NOTHING` then a read: two first messages racing each other
/// both end up in the ONE thread the unique constraint allows.
///
/// # Errors
/// A database failure.
pub async fn get_or_create_discussion(
    pool: &PgPool,
    anchor_type: &str,
    anchor_id: &str,
    username: &str,
) -> Result<DiscussionRecord, PipelineRepoError> {
    sqlx::query(
        "INSERT INTO discussions (anchor_type, anchor_id, username) VALUES ($1, $2, $3) \
         ON CONFLICT (anchor_type, anchor_id, username) DO NOTHING",
    )
    .bind(anchor_type)
    .bind(anchor_id)
    .bind(username)
    .execute(pool)
    .await?;
    find_discussion(pool, anchor_type, anchor_id, username)
        .await?
        .ok_or_else(|| PipelineRepoError::from(sqlx::Error::RowNotFound))
}

/// Every thread on an anchor, with counts as the viewer sees them.
///
/// # Errors
/// A database failure.
pub async fn list_threads(
    pool: &PgPool,
    anchor_type: &str,
    anchor_id: &str,
    viewer: &str,
) -> Result<Vec<ThreadSummary>, PipelineRepoError> {
    sqlx::query_as::<_, ThreadSummary>(
        "SELECT d.id, d.username, d.created_at, \
                COUNT(m.seq) FILTER (WHERE m.rendered_text IS NOT NULL) AS visible_count, \
                COALESCE(MAX(m.seq), 0) AS last_seq, \
                COALESCE(r.last_seen_seq, 0) AS seen_seq, \
                COUNT(m.seq) FILTER (WHERE m.rendered_text IS NOT NULL \
                    AND m.seq > COALESCE(r.last_seen_seq, 0)) AS unread, \
                (SELECT m2.rendered_text FROM discussion_messages m2 \
                  WHERE m2.discussion_id = d.id AND m2.rendered_text IS NOT NULL \
                  ORDER BY m2.seq DESC LIMIT 1) AS last_text \
         FROM discussions d \
         LEFT JOIN discussion_messages m ON m.discussion_id = d.id \
         LEFT JOIN discussion_reads r ON r.discussion_id = d.id AND r.username = $3 \
         WHERE d.anchor_type = $1 AND d.anchor_id = $2 \
         GROUP BY d.id, d.username, d.created_at, r.last_seen_seq \
         ORDER BY d.created_at, d.username",
    )
    .bind(anchor_type)
    .bind(anchor_id)
    .bind(viewer)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// A thread's messages, oldest first.
///
/// # Errors
/// A database failure.
pub async fn list_messages(
    pool: &PgPool,
    discussion_id: Uuid,
) -> Result<Vec<MessageRecord>, PipelineRepoError> {
    sqlx::query_as::<_, MessageRecord>(
        "SELECT seq, role, content, rendered_text, citations, model, stop_reason, failure, \
                created_at \
         FROM discussion_messages WHERE discussion_id = $1 ORDER BY seq",
    )
    .bind(discussion_id)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// Append one message; returns its seq.
///
/// The seq is computed in the INSERT itself. Two appends racing on one thread
/// collide on `discussion_messages_seq_unique` and the loser fails loudly rather
/// than interleaving — a person does not type two messages in one thread at once.
///
/// # Errors
/// A database failure, including that collision.
pub async fn append_message(
    pool: &PgPool,
    discussion_id: Uuid,
    m: &NewMessage,
) -> Result<i32, PipelineRepoError> {
    let (seq,): (i32,) = sqlx::query_as(
        "INSERT INTO discussion_messages \
            (discussion_id, seq, role, content, rendered_text, citations, model, stop_reason, \
             input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens, ms, failure) \
         SELECT $1, COALESCE(MAX(seq), 0) + 1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13 \
           FROM discussion_messages WHERE discussion_id = $1 \
         RETURNING seq",
    )
    .bind(discussion_id)
    .bind(m.role)
    .bind(&m.content)
    .bind(&m.rendered_text)
    .bind(&m.citations)
    .bind(&m.model)
    .bind(&m.stop_reason)
    .bind(m.input_tokens)
    .bind(m.output_tokens)
    .bind(m.cache_creation_tokens)
    .bind(m.cache_read_tokens)
    .bind(m.ms)
    .bind(&m.failure)
    .fetch_one(pool)
    .await?;
    Ok(seq)
}

/// Successful assistant replies in a thread — what the turn cap counts.
///
/// # Errors
/// A database failure.
pub async fn assistant_reply_count(
    pool: &PgPool,
    discussion_id: Uuid,
) -> Result<i64, PipelineRepoError> {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM discussion_messages \
         WHERE discussion_id = $1 AND role = 'assistant' AND rendered_text IS NOT NULL",
    )
    .bind(discussion_id)
    .fetch_one(pool)
    .await?;
    Ok(n)
}

/// Record that `username` has seen `discussion_id` up to `seq`. Never moves back.
///
/// # Errors
/// A database failure.
pub async fn mark_read(
    pool: &PgPool,
    discussion_id: Uuid,
    username: &str,
    seq: i32,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "INSERT INTO discussion_reads (discussion_id, username, last_seen_seq) VALUES ($1, $2, $3) \
         ON CONFLICT (discussion_id, username) \
         DO UPDATE SET last_seen_seq = GREATEST(discussion_reads.last_seen_seq, EXCLUDED.last_seen_seq)",
    )
    .bind(discussion_id)
    .bind(username)
    .bind(seq)
    .execute(pool)
    .await?;
    Ok(())
}

/// Question threads whose question no longer exists (ruling D5's boot count).
///
/// # Errors
/// A database failure.
pub async fn orphan_question_threads(pool: &PgPool) -> Result<i64, PipelineRepoError> {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM discussions d \
         WHERE d.anchor_type = 'question' \
           AND NOT EXISTS (SELECT 1 FROM practice_questions q WHERE q.id::text = d.anchor_id)",
    )
    .fetch_one(pool)
    .await?;
    Ok(n)
}

/// Every stored document that has text, with its pages — the chat's corpus
/// (GO ruling on STOP 1: the whole corpus, cited from directly).
///
/// # Errors
/// A database failure.
pub async fn load_corpus(pool: &PgPool) -> Result<Vec<CorpusDocument>, PipelineRepoError> {
    let rows: Vec<(String, String, Option<NaiveDate>, i32, String)> = sqlx::query_as(
        "SELECT d.id, d.title, d.document_date, t.page_number, t.text_content \
         FROM documents d JOIN document_text t ON t.document_id = d.id \
         ORDER BY d.id, t.page_number",
    )
    .fetch_all(pool)
    .await?;
    let mut out: Vec<CorpusDocument> = Vec::new();
    for (id, title, date, page, text) in rows {
        match out.last_mut() {
            Some(doc) if doc.id == id => doc.pages.push((page, text)),
            _ => out.push(CorpusDocument {
                id,
                title,
                document_date: date,
                pages: vec![(page, text)],
            }),
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "chat_discussions_live_tests.rs"]
mod live_tests;
