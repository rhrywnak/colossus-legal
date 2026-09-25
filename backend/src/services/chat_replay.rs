//! Replaying a stored question-chat thread to the model — what is SENT, never
//! what is stored.
//!
//! Every turn resends the whole conversation so far. The stored rows are kept
//! exactly as the provider produced them (thinking signatures, tool ids and
//! compaction blocks depend on that — see `colossus_chat::accumulate`). One field
//! cannot be resent as stored: the provider's `citations` on a reply's text
//! blocks.
//!
//! ## Why the citations are stripped on the way out
//!
//! A provider citation names its source by POSITION — `document_index` N is "the
//! Nth document in the list sent with that request" — and a character range
//! inside it. The list is not fixed: a document ingested later can land in the
//! middle of it, and every position after it moves up by one. Replayed as stored,
//! an old citation then names a different document. When its range runs past
//! the end of that document the provider refuses the whole request with an HTTP
//! 400 (thread 16a656b7, 2026-09-25: "Start index 38268 is beyond document length
//! 18815"); when the range happens to fit, the model is silently shown a
//! quotation pinned to the wrong document (thread 3e1f13eb, 10 of 14).
//!
//! Why strip rather than re-point each citation by title: the model needs the
//! reply's TEXT to follow the conversation, not the positions; re-pointing would
//! be a second copy of the document list's ordering rules to keep in step.
//!
//! Domain note: Marie's quotation cards do not come from here. They were resolved
//! to document ids when the reply was written and are read from the stored
//! `citations` column (`chat_question_view::stored_cards`), so stripping the
//! provider's positional copy changes nothing she sees.

use colossus_chat::{Message, Role};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::repositories::pipeline_repository::chat_discussions::{list_messages, MessageRecord};
use crate::services::chat_question_error::{store, ChatRunError};

// STRUCTURAL: Anthropic Messages API vocabulary (wire protocol, not settings).
const FIELD_CITATIONS: &str = "citations";
const FIELD_TYPE: &str = "type";
const ROLE_ASSISTANT: &str = "assistant";
/// Signed blocks: the provider checks them byte for byte on replay, so they are
/// never edited — even though neither type carries citations today.
const SIGNED_BLOCKS: [&str; 2] = ["thinking", "redacted_thinking"];

/// The stored thread as the model must see it again.
///
/// Reads the rows and hands them to [`replayable`]; see there for what changes.
/// Takes the pipeline pool rather than `AppState`: the thread is all it needs,
/// and a pool is what a test can aim at a dead port to prove the error path.
///
/// # Errors
/// [`ChatRunError`] (via `store`) when the thread cannot be read — logged with
/// the operation and question id by the handler that receives it.
#[tracing::instrument(skip(pool), fields(question_id = %question_id, discussion_id = %discussion_id))]
pub async fn replay_history(
    pool: &PgPool,
    question_id: Uuid,
    discussion_id: Uuid,
) -> Result<Vec<Message>, ChatRunError> {
    let rows = list_messages(pool, discussion_id)
        .await
        .map_err(store("list_messages", question_id))?;
    Ok(replayable(discussion_id, rows))
}

/// Stored rows → the history to send.
///
/// - failed assistant turns are dropped (a failure marker has no content to
///   replay); her messages around them stay, so several user messages can sit
///   back to back — the provider merges same-role neighbours, and every turn
///   already sends two (the documents message, then her first message);
/// - every assistant block loses its positional `citations` ([`strip_positions`]);
/// - user messages (her text, tool results) are sent as stored.
///
/// Pure — no database — so the tests drive it with rows built in memory.
/// `discussion_id` is used only to name a malformed row in the log.
pub fn replayable(discussion_id: Uuid, rows: Vec<MessageRecord>) -> Vec<Message> {
    rows.into_iter()
        .filter(|m| m.failure.is_none())
        .map(|m| {
            let assistant = m.role == ROLE_ASSISTANT;
            let mut content = content_blocks(discussion_id, &m);
            if assistant {
                content.iter_mut().for_each(strip_positions);
            }
            Message {
                role: if assistant {
                    Role::Assistant
                } else {
                    Role::User
                },
                content,
            }
        })
        .collect()
}

/// A row's content blocks. `append_message` always stores a JSON array (from a
/// `Vec<Value>`), so anything else is a store defect. It is logged with the row
/// that holds it and replayed as an empty message rather than guessed at. The
/// provider then refuses the request, and the stored failure names the message.
fn content_blocks(discussion_id: Uuid, m: &MessageRecord) -> Vec<Value> {
    match m.content.as_array() {
        Some(blocks) => blocks.clone(),
        None => {
            tracing::warn!(%discussion_id, seq = m.seq, role = %m.role, content = %m.content,
                "question chat replay: a stored message's content is not a JSON array; \
                 it is sent empty — inspect this row in discussion_messages");
            Vec::new()
        }
    }
}

/// Remove the provider's positional `citations` from one assistant block, in
/// place. The block's `text` (and every other field) is left byte-identical.
///
/// ## Rust Learning: editing a `serde_json::Value` in place
///
/// `Value::as_object_mut()` returns `Option<&mut Map<String, Value>>` — a
/// mutable borrow of the object inside the enum, or `None` when the value is not
/// an object. `Map::remove` then drops the key without rebuilding the block.
/// Taking `&mut Value` (rather than returning a new `Value`) is what lets the
/// caller use `iter_mut().for_each(strip_positions)` over the content vector
/// with no cloning.
fn strip_positions(block: &mut Value) {
    let signed = block
        .get(FIELD_TYPE)
        .and_then(Value::as_str)
        .is_some_and(|t| SIGNED_BLOCKS.contains(&t));
    if signed {
        return;
    }
    // A block that is not a JSON object has no fields, so it has no citations
    // to remove. It is sent as stored, and the provider names it if it is bad.
    if let Some(fields) = block.as_object_mut() {
        fields.remove(FIELD_CITATIONS);
    }
}

#[cfg(test)]
#[path = "chat_replay_tests.rs"]
mod tests;
