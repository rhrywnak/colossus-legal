//! The reviewers' queue, per deck — now DERIVED from the per-item seen record.
//!
//! CC_TASK_FOR_YOU_v1 L2. The name, the row shape and the four call sites are
//! exactly what they were; the body is gone, and what is left is an adapter over
//! `waiting_items::waiting_counts`.
//!
//! ## What this module used to be, and why it stopped
//!
//! It held `practice_review_cursor`: one row per (person, deck) carrying the
//! moment that person last pressed Done reviewing, and a query that counted
//! events newer than it. That watermark could say "this deck holds something
//! new" and could never say WHICH — so Marie, 185 questions into eleven decks,
//! had no way to find the one Chuck had written on. L0 replaced it with a row
//! per (person, ITEM) seen; this is the layer where the last reader of the old
//! table stops reading it.
//!
//! `mark_reviewed` is gone with it. The press writes seen rows now
//! (`api::practice_review_cursor::put_review_cursor` → `item_seen::mark_seen`),
//! and the table itself stays in the schema, unread, with the `COMMENT ON TABLE`
//! recording its retirement until a later release drops it (ruling Q6).
//!
//! ## ⚑ The count is now PER VIEWER, where it used to be one number for all
//!
//! This is the consequence of the whole design and it is worth stating plainly.
//! CC_TASK_SIMPLE_COUNTS_v1 made this number global — every viewer saw the
//! reviewers' backlog — because a shared watermark is the only thing a shared
//! count can be derived from. Read-state is per person now, so the count is what
//! is waiting FOR THE PERSON ASKING. Chuck's press clears Chuck's queue; Roman,
//! reviewing the same deck, still has his own to read.
//!
//! That is the point rather than a side effect: a personal inbox whose count
//! belonged to somebody else would be answering a different question from the
//! one the page's title asks. What did NOT change is WHOSE WORK counts — the
//! exclusion legs still read the stored `practice_reviewer_usernames` list
//! (ruling R2), and the war room's named lines are still display only.
//!
//! ## CRITICAL — the pipeline pool
//!
//! `practice_item_seen` lives in `colossus_legal_v2`: `&state.pipeline_pool`.

use sqlx::PgPool;
use uuid::Uuid;

use super::waiting_items::{waiting_counts, WaitingQuery, WaitingScope, WaitingSide};
use super::PipelineRepoError;

/// The review queue for one scenario: how many items wait, and since when.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct AwaitingReviewRow {
    pub scenario_id: Uuid,
    /// Items on the reviewers' side this viewer has not seen.
    pub awaiting: i64,
    /// When the OLDEST of those was written; `None` when nothing waits.
    pub oldest: Option<chrono::DateTime<chrono::Utc>>,
}

/// What waits for THIS viewer on each of these decks, on the reviewers' side.
///
/// ## Domain note: the same predicate as the page, asked per deck
///
/// The war room's pill, the deck's review bar and the For You list are three
/// readings of ONE query (`waiting_items`). That is deliberate: a count that
/// disagreed with the list it links to would send somebody to a page that
/// showed them nothing, and the two could only disagree if they were two
/// queries. They are not.
///
/// `reviewers` is still the stored list, and still decides whose work does not
/// wait — never who may read this number.
///
/// # Errors
/// [`PipelineRepoError`] for a failed statement.
pub async fn awaiting_review(
    pool: &PgPool,
    scenario_ids: &[Uuid],
    viewer: &str,
    reviewers: &[String],
) -> Result<Vec<AwaitingReviewRow>, PipelineRepoError> {
    let rows = waiting_counts(
        pool,
        &WaitingQuery {
            scenario_ids,
            viewer,
            reviewers,
            side: WaitingSide::Reviewers,
            scope: WaitingScope::Unseen,
        },
    )
    .await?;
    // One row per scenario ASKED FOR comes back from `waiting_counts` — the
    // `unnest`-first shape — so this mapping neither adds rows nor drops one,
    // and a deck holding nothing is still a zero rather than a missing entry.
    Ok(rows
        .into_iter()
        .map(|row| AwaitingReviewRow {
            scenario_id: row.scenario_id,
            awaiting: row.waiting,
            oldest: row.oldest,
        })
        .collect())
}

#[cfg(test)]
#[path = "review_cursor_live_tests.rs"]
mod live_tests;
