//! The most recent successful chat reply across every thread: when it was, and
//! whose thread it was in. Read-only; the Admin "Chat case file" box shows it.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::PipelineRepoError;

/// When the latest successful reply was stored, and the thread owner's login.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct LastChatTurn {
    pub at: DateTime<Utc>,
    pub username: String,
}

/// The latest successful assistant reply, or `None` when nobody has chatted.
///
/// "Successful" is the same test the turn cap uses (`assistant_reply_count`):
/// an assistant row with shown text. `failure IS NULL` is added so a failure
/// marker can never count as a chat. The intermediate tool-use rows of a turn
/// have no shown text, so the row found is the turn's final reply.
///
/// # Errors
/// A database failure.
pub async fn last_chat_turn(pool: &PgPool) -> Result<Option<LastChatTurn>, PipelineRepoError> {
    let row: Option<LastChatTurn> = sqlx::query_as(
        "SELECT m.created_at AS at, d.username \
           FROM discussion_messages m \
           JOIN discussions d ON d.id = m.discussion_id \
          WHERE m.role = 'assistant' AND m.rendered_text IS NOT NULL AND m.failure IS NULL \
          ORDER BY m.created_at DESC \
          LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
