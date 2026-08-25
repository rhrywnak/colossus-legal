//! Chronology links, notes, history, and the target-resolution read.
//!
//! Split from `chronology.rs` for Rule 17; the row types live there and this
//! module only queries them. Pipeline pool, as everywhere in this repository.
//!
//! ## Why resolution is a READ and not a foreign key
//!
//! `chronology_event_links` has no FK to `documents`, because `target_type`
//! decides which store the target lives in and no single constraint can express
//! that. So "does this link point at something real?" is answered at read time,
//! by [`existing_document_ids`], and returned to the frontend as DATA. A dead
//! link renders as "no document"; it is never a 500 and it is never silently
//! dropped. That is the defect this whole design was written after: ten of the
//! eleven links in the old JSON pointed at ids that did not exist, and the page
//! rendered every one of them as a live link.

use std::collections::HashSet;

use uuid::Uuid;

use super::chronology::{ChronologyHistoryRow, ChronologyLinkRow, ChronologyNoteRow};
use super::PipelineRepoError;

/// Shared projection for link reads.
const LINK_COLUMNS: &str =
    "event_id, target_type, target_id, label, pinpoint, created_by, created_at";

/// Every link belonging to one case's live events.
///
/// One query for the whole page rather than one per event: the list endpoint
/// renders every event's link chips, and 22 round trips to save a join is the
/// N+1 the design's volume assumption does not need.
pub async fn list_links_for_case(
    executor: impl sqlx::PgExecutor<'_>,
    case_id: &str,
) -> Result<Vec<ChronologyLinkRow>, PipelineRepoError> {
    // Spelled out with the alias rather than derived from LINK_COLUMNS: a
    // string transform that inserts "l." would be one clever line nobody could
    // read at a glance, and the column set is pinned by the same FromRow type
    // either way.
    let rows = sqlx::query_as::<_, ChronologyLinkRow>(
        "SELECT l.event_id, l.target_type, l.target_id, l.label, l.pinpoint, \
                l.created_by, l.created_at \
         FROM chronology_event_links l \
         JOIN chronology_events e ON e.id = l.event_id \
         WHERE e.case_id = $1 AND e.deleted_at IS NULL \
         ORDER BY l.event_id, l.target_type, l.target_id",
    )
    .bind(case_id)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// Every link on one event.
pub async fn list_links_for_event(
    executor: impl sqlx::PgExecutor<'_>,
    event_id: Uuid,
) -> Result<Vec<ChronologyLinkRow>, PipelineRepoError> {
    let sql = format!(
        "SELECT {LINK_COLUMNS} FROM chronology_event_links \
         WHERE event_id = $1 ORDER BY target_type, target_id"
    );
    let rows = sqlx::query_as::<_, ChronologyLinkRow>(&sql)
        .bind(event_id)
        .fetch_all(executor)
        .await?;
    Ok(rows)
}

/// `(event_id, live note count)` for one case, for the list endpoint's badges.
///
/// Only events that HAVE notes come back. The caller treats a missing event as
/// zero, which is the same fact expressed with fewer rows.
pub async fn note_counts_for_case(
    executor: impl sqlx::PgExecutor<'_>,
    case_id: &str,
) -> Result<Vec<(Uuid, i64)>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, (Uuid, i64)>(
        "SELECT n.event_id, COUNT(*) FROM chronology_event_notes n \
         JOIN chronology_events e ON e.id = n.event_id \
         WHERE e.case_id = $1 AND e.deleted_at IS NULL AND n.deleted_at IS NULL \
         GROUP BY n.event_id",
    )
    .bind(case_id)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// Every live note on one event, oldest first — the order they were written in.
pub async fn list_notes_for_event(
    executor: impl sqlx::PgExecutor<'_>,
    event_id: Uuid,
) -> Result<Vec<ChronologyNoteRow>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ChronologyNoteRow>(
        "SELECT id, event_id, note, created_by, created_at \
         FROM chronology_event_notes \
         WHERE event_id = $1 AND deleted_at IS NULL ORDER BY created_at, id",
    )
    .bind(event_id)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// Every history entry for one event, oldest first.
///
/// Empty for every event in Phase A: nothing writes history until the write
/// endpoints land in Phase C. An empty list and a missing table are NOT the same
/// observable — the table exists, so this returns `Ok(vec![])` and the detail
/// endpoint renders "no changes recorded", rather than failing.
pub async fn list_history_for_event(
    executor: impl sqlx::PgExecutor<'_>,
    event_id: Uuid,
) -> Result<Vec<ChronologyHistoryRow>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ChronologyHistoryRow>(
        "SELECT id, event_id, action, snapshot, changed_by, changed_at \
         FROM chronology_event_history WHERE event_id = $1 ORDER BY changed_at, id",
    )
    .bind(event_id)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// Which of `ids` exist in the `documents` table.
///
/// ## Rust Learning: `= ANY($1)` instead of building an IN list
///
/// Postgres accepts an array parameter and `= ANY($1)` against it, so one bound
/// `&[String]` serves any number of ids with no string-built SQL and no
/// injection surface. Binding a `Vec<String>` sends a `text[]`.
///
/// An empty input short-circuits: `= ANY('{}')` is a legal query that returns
/// nothing, but asking the database a question whose answer is known is a round
/// trip for no reason.
pub async fn existing_document_ids(
    executor: impl sqlx::PgExecutor<'_>,
    ids: &[String],
) -> Result<HashSet<String>, PipelineRepoError> {
    if ids.is_empty() {
        return Ok(HashSet::new());
    }
    let found = sqlx::query_scalar::<_, String>("SELECT id FROM documents WHERE id = ANY($1)")
        .bind(ids)
        .fetch_all(executor)
        .await?;
    Ok(found.into_iter().collect())
}
