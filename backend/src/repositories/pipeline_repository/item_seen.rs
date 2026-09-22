//! The seen record: who has looked at which item (CC_TASK_FOR_YOU_v1 L0).
//!
//! One row per (person, item). An ITEM is a row that already exists — the
//! current answer to a question, a standing note, or a reworded/edited deck
//! change — so nothing here is written when an event happens, only when
//! somebody looks at one.
//!
//! This is the finer-grained replacement for `practice_review_cursor`'s
//! per-deck watermark. The watermark could say "this deck holds something new";
//! it could never say WHICH question, because a timestamp has no memory of what
//! it swept past. The counts stay derived either way: there is no stored counter
//! in this module to drift out of step with the answers.
//!
//! ## CRITICAL — the pipeline pool
//!
//! `practice_item_seen` lives in `colossus_legal_v2`: `&state.pipeline_pool`.

use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

/// One item a person can have seen.
///
/// ## Rust Learning: an enum instead of a `(&str, Uuid)` pair
///
/// The three kinds could have been carried as a string and an id, and the
/// compiler would have had nothing to say about `("anser", id)`. As an enum the
/// set of kinds is closed: a fourth one cannot be spelled at all, and the
/// `match` in [`mark_seen`] below stops compiling the day somebody adds one —
/// which is exactly when a silent gap would otherwise open in the sweep.
///
/// It mirrors the table's own three-nullable-columns shape, and the `CHECK
/// (num_nonnulls(...) = 1)` on that table is the database saying the same thing
/// in its own language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemRef {
    /// `practice_answers.id` — the current answer to a question.
    Answer(Uuid),
    /// `practice_notes.id` — a note, while it stands.
    Note(Uuid),
    /// `practice_deck_changes.id` — a rewording or an edit.
    Change(Uuid),
}

/// Record that `user_id` has seen these items. Returns how many rows were NEW.
///
/// ## Domain note: the sweep writes what the page SHOWED, never "everything now"
///
/// "Done reviewing marks exactly the items that page showed — never an item that
/// arrived after the page loaded" (ruling 1). That is why this takes a list of
/// ids rather than a scenario and a moment: an item written between the render
/// and the press is not in the list, so it cannot be swept away unread. A
/// watermark could not make that promise — `now()` covers everything, including
/// the thing nobody has read.
///
/// ## Rust Learning: `ON CONFLICT DO NOTHING`
///
/// A second visit collides with the first row on the per-leg unique index.
/// `DO NOTHING` turns that collision into a no-op instead of an error, which
/// makes re-reading a page harmless — and it deliberately KEEPS the first
/// `seen_at` rather than moving it, because the moment worth recording is when
/// this person first laid eyes on the item. (`DO UPDATE` would have overwritten
/// it with the latest glance, which answers a question nobody asked.)
///
/// The return value is the count of rows actually inserted, so a caller can log
/// "swept 12 of 17, five were already seen" rather than guessing. An empty
/// `items` slice writes nothing and returns 0 — a legitimate state (a page with
/// nothing waiting on it), reported as a number rather than swallowed.
///
/// # Errors
/// [`PipelineRepoError::Database`] when a statement fails — including the
/// foreign keys refusing an id that names no answer, note or change, and the
/// `CHECK` refusing a blank `user_id`. Both are loud on purpose: an id the
/// caller invented must not be recorded as read.
pub async fn mark_seen(
    pool: &PgPool,
    user_id: &str,
    items: &[ItemRef],
) -> Result<u64, PipelineRepoError> {
    if items.is_empty() {
        return Ok(0);
    }
    // One pass, three baskets. The `match` is what makes the enum worth having:
    // add a fourth kind and this stops compiling here, at the write, rather than
    // silently dropping that kind on the floor.
    let (mut answers, mut notes, mut changes) = (Vec::new(), Vec::new(), Vec::new());
    for item in items {
        match item {
            ItemRef::Answer(id) => answers.push(*id),
            ItemRef::Note(id) => notes.push(*id),
            ItemRef::Change(id) => changes.push(*id),
        }
    }
    // ONE statement for all three legs rather than three round trips: the sweep
    // is either recorded or it is not, and a partial sweep interrupted between
    // statements would leave a page half-read with nothing saying so.
    let done = sqlx::query(
        // The `NULL::uuid` casts are not decoration: a `UNION ALL` of three
        // branches has to agree on a type for each column, and a bare `NULL`
        // leaves Postgres to infer one from the first branch. Naming the type
        // makes the statement say what it means — and keeps the SQL-shape guard
        // able to tell a literal from a column name.
        "INSERT INTO practice_item_seen (user_id, answer_id, note_id, change_id) \
         SELECT $1, a.id, NULL::uuid, NULL::uuid FROM unnest($2::uuid[]) AS a(id) \
         UNION ALL SELECT $1, NULL::uuid, n.id, NULL::uuid FROM unnest($3::uuid[]) AS n(id) \
         UNION ALL SELECT $1, NULL::uuid, NULL::uuid, c.id FROM unnest($4::uuid[]) AS c(id) \
         ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    // ## Rust Learning: binding a `Vec<Uuid>` as a Postgres array
    //
    // sqlx encodes it as `uuid[]`, which `unnest` turns back into rows. One bind
    // per leg, and no SQL assembled by string concatenation — the alternative (a
    // `VALUES` list built in Rust) would put caller-supplied ids into the
    // statement text itself.
    .bind(&answers)
    .bind(&notes)
    .bind(&changes)
    .execute(pool)
    .await?;
    Ok(done.rows_affected())
}

/// How many items this person has seen. For proofs and for operator questions
/// ("did the sweep actually write anything?"), never for a screen.
///
/// # Errors
/// [`PipelineRepoError::Database`] for a failed statement.
pub async fn seen_count(pool: &PgPool, user_id: &str) -> Result<i64, PipelineRepoError> {
    let row: (i64,) = sqlx::query_as("SELECT count(*) FROM practice_item_seen WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}
