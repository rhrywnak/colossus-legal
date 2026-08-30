//! Row types and READS for timeline subsets (T1.3).
//!
//! TIMELINE_SUBSET_DESIGN_v1 §4. Every table here lives in `colossus_legal_v2`,
//! so every call takes the PIPELINE pool — `&state.pipeline_pool`, never
//! `state.pg_pool`. The sibling `chronology_subset_write` holds every statement
//! that CHANGES one of these tables and is the one write path's floor; nothing
//! in this file writes anything.
//!
//! ## ⚑ THE JOIN IS THE WHOLE POINT
//!
//! `chronology_subset_events` carries an event id and nothing about the event.
//! Every read that shows a subset's events therefore joins `chronology_events`,
//! and it joins it WITHOUT the usual `deleted_at IS NULL` filter — because a
//! removed event must come back marked rather than silently missing (design R1).
//! That is the one place this module deliberately breaks the chronology reader's
//! habit, and [`list_subset_event_ids`] plus `chronology_write::get_event_any_state`
//! are how it does it.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::PipelineRepoError;

/// One subset, as `chronology_subsets` stores it.
///
/// `created_by` / `updated_by` are `String` and not `Option<String>`, unlike
/// `ChronologyEventRow`'s: that table had to tolerate rows the seed one-shot
/// wrote before any human existed, and a subset can only be born through the
/// guarded write path, which always has a signed-in user.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChronologySubsetRow {
    pub id: Uuid,
    pub case_slug: String,
    pub name: String,
    pub description: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_by: String,
    pub updated_at: DateTime<Utc>,
    /// NULL = live (chronology R10).
    pub deleted_at: Option<DateTime<Utc>>,
}

/// One reference from a subset to an event, with the author's note.
///
/// ⚑ There is no title, no date and no fact here, and there never will be
/// (design §4). Whoever adds one has turned a reference into a copy.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SubsetEventRefRow {
    pub subset_id: Uuid,
    pub event_id: Uuid,
    pub position: i32,
    pub note: String,
}

/// How many events one subset references, and how many of those are gaps.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SubsetCountsRow {
    pub subset_id: Uuid,
    pub event_count: i64,
    /// Events whose `deleted_at` is set — removed from the chronology, still
    /// referenced by the story.
    pub gap_count: i64,
}

/// Which scenario carries which subset, and under what code.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SubsetCarrierRow {
    pub subset_id: Uuid,
    /// The scenario's stored ordinal. Rendered `S-{n}` by
    /// `domain::scenario_code` — the backend owns the spelling, never a screen.
    pub code_ordinal: i32,
    pub position: i32,
}

/// One subset attached to one scenario, with the two counts the button reads.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ScenarioSubsetRow {
    pub subset_id: Uuid,
    pub name: String,
    pub position: i32,
    pub event_count: i64,
    pub gap_count: i64,
}

/// The projection every subset read shares, so two queries cannot disagree about
/// the column set [`ChronologySubsetRow`] expects.
const SUBSET_COLUMNS: &str = "id, case_slug, name, description, created_by, created_at, \
     updated_by, updated_at, deleted_at";

/// Every LIVE subset for one case, ordered by name.
///
/// ## Why by name and not by creation date
///
/// The home section is a list a person looks something UP in — "where is The
/// $50,000?" — not a feed. `lower(name)` so case does not scatter the order, and
/// `id` breaks ties so two reads of unchanged data never swap two rows (the
/// live-name unique index makes a real tie impossible; the tiebreak is for the
/// day somebody drops that index).
pub async fn list_subsets(
    executor: impl sqlx::PgExecutor<'_>,
    case_slug: &str,
) -> Result<Vec<ChronologySubsetRow>, PipelineRepoError> {
    let sql = format!(
        "SELECT {SUBSET_COLUMNS} FROM chronology_subsets \
         WHERE case_slug = $1 AND deleted_at IS NULL \
         ORDER BY lower(name), id"
    );
    let rows = sqlx::query_as::<_, ChronologySubsetRow>(&sql)
        .bind(case_slug)
        .fetch_all(executor)
        .await?;
    Ok(rows)
}

/// One subset by id, WHATEVER STATE it is in, or `None` when there is no such id.
///
/// Any state, not just live: the Undo endpoint has to be able to find a deleted
/// subset, and a caller that must refuse one says so itself — which keeps "gone"
/// and "one press from back" two different answers.
pub async fn get_subset_any_state(
    executor: impl sqlx::PgExecutor<'_>,
    id: Uuid,
) -> Result<Option<ChronologySubsetRow>, PipelineRepoError> {
    let sql = format!("SELECT {SUBSET_COLUMNS} FROM chronology_subsets WHERE id = $1");
    let row = sqlx::query_as::<_, ChronologySubsetRow>(&sql)
        .bind(id)
        .fetch_optional(executor)
        .await?;
    Ok(row)
}

/// The id of a LIVE subset in this case whose name matches, case-insensitively.
///
/// The read behind the 409. The partial unique index would refuse the write
/// anyway, and it would come back as a constraint violation this codebase turns
/// into a 500 — an operator paged over somebody typing a name twice. This is the
/// answer a human reads; the index stays as the backstop it was designed to be.
///
/// `exclude` is the subset being renamed, so renaming one to its own name is not
/// a clash with itself.
pub async fn live_subset_named(
    executor: impl sqlx::PgExecutor<'_>,
    case_slug: &str,
    name: &str,
    exclude: Option<Uuid>,
) -> Result<Option<Uuid>, PipelineRepoError> {
    let id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM chronology_subsets \
         WHERE case_slug = $1 AND lower(name) = lower($2) AND deleted_at IS NULL \
           AND ($3::uuid IS NULL OR id <> $3) \
         LIMIT 1",
    )
    .bind(case_slug)
    .bind(name)
    .bind(exclude)
    .fetch_optional(executor)
    .await?;
    Ok(id)
}

/// The event references of one subset, in story order.
pub async fn list_subset_event_ids(
    executor: impl sqlx::PgExecutor<'_>,
    subset_id: Uuid,
) -> Result<Vec<SubsetEventRefRow>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, SubsetEventRefRow>(
        "SELECT subset_id, event_id, position, note \
         FROM chronology_subset_events WHERE subset_id = $1 \
         ORDER BY position, event_id",
    )
    .bind(subset_id)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// The two counts for a set of subsets, in ONE query.
///
/// ## Why a LEFT JOIN and a FILTER
///
/// A subset with no events must come back with `0, 0` rather than not come back
/// at all — the home section renders it, and a missing row would make an empty
/// story indistinguishable from one this query failed to see. The `FILTER
/// (WHERE e.deleted_at IS NOT NULL)` counts the gaps in the same pass, so the
/// list read is one round trip whatever the number of subsets.
pub async fn subset_counts(
    executor: impl sqlx::PgExecutor<'_>,
    subset_ids: &[Uuid],
) -> Result<Vec<SubsetCountsRow>, PipelineRepoError> {
    if subset_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, SubsetCountsRow>(
        "SELECT s.id AS subset_id, \
                COUNT(se.event_id) AS event_count, \
                COUNT(*) FILTER (WHERE e.deleted_at IS NOT NULL) AS gap_count \
         FROM chronology_subsets s \
         LEFT JOIN chronology_subset_events se ON se.subset_id = s.id \
         LEFT JOIN chronology_events e ON e.id = se.event_id \
         WHERE s.id = ANY($1) \
         GROUP BY s.id",
    )
    .bind(subset_ids)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// Which scenarios carry these subsets, in attachment order.
pub async fn carriers_for_subsets(
    executor: impl sqlx::PgExecutor<'_>,
    subset_ids: &[Uuid],
) -> Result<Vec<SubsetCarrierRow>, PipelineRepoError> {
    if subset_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, SubsetCarrierRow>(
        "SELECT ss.subset_id, sc.code_ordinal, ss.position \
         FROM scenario_subsets ss \
         JOIN scenarios sc ON sc.scenario_id = ss.scenario_id \
         WHERE ss.subset_id = ANY($1) \
         ORDER BY ss.position, sc.code_ordinal",
    )
    .bind(subset_ids)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// The subsets one scenario carries, in `position` order, with their counts.
///
/// A DELETED subset is excluded: detaching nothing is the design's rule
/// (`DELETE /timeline/subsets/:id` leaves the link rows alone), and the way that
/// stays coherent is that a deleted subset is simply not read. Undoing the
/// delete brings the attachment back with it, which is what "detaches nothing"
/// is for.
pub async fn list_scenario_subsets(
    executor: impl sqlx::PgExecutor<'_>,
    scenario_id: Uuid,
) -> Result<Vec<ScenarioSubsetRow>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ScenarioSubsetRow>(
        "SELECT s.id AS subset_id, s.name, ss.position, \
                COUNT(se.event_id) AS event_count, \
                COUNT(*) FILTER (WHERE e.deleted_at IS NOT NULL) AS gap_count \
         FROM scenario_subsets ss \
         JOIN chronology_subsets s ON s.id = ss.subset_id \
         LEFT JOIN chronology_subset_events se ON se.subset_id = s.id \
         LEFT JOIN chronology_events e ON e.id = se.event_id \
         WHERE ss.scenario_id = $1 AND s.deleted_at IS NULL \
         GROUP BY s.id, s.name, ss.position \
         ORDER BY ss.position, lower(s.name)",
    )
    .bind(scenario_id)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// Which of these event ids exist in this case, whatever state they are in.
///
/// The read behind the 422. Deleted events count as existing: a story may
/// deliberately reference an event somebody removed, and refusing to reference
/// one would make the gap unrepresentable — which is the state the design
/// specifically wants visible.
pub async fn existing_event_ids_in_case(
    executor: impl sqlx::PgExecutor<'_>,
    case_slug: &str,
    event_ids: &[Uuid],
) -> Result<Vec<Uuid>, PipelineRepoError> {
    if event_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM chronology_events WHERE case_slug = $1 AND id = ANY($2)",
    )
    .bind(case_slug)
    .bind(event_ids)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// The position an appended attachment takes: one past the highest in use.
///
/// `COALESCE(MAX(position), -1) + 1` so the first attachment is `0`, matching
/// the column's own default — the two cannot disagree about what "first" means.
pub async fn next_scenario_subset_position(
    executor: impl sqlx::PgExecutor<'_>,
    scenario_id: Uuid,
) -> Result<i32, PipelineRepoError> {
    let next = sqlx::query_scalar::<_, i32>(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM scenario_subsets WHERE scenario_id = $1",
    )
    .bind(scenario_id)
    .fetch_one(executor)
    .await?;
    Ok(next)
}

/// Whether this scenario already carries this subset — the read behind the 409.
pub async fn is_subset_attached(
    executor: impl sqlx::PgExecutor<'_>,
    scenario_id: Uuid,
    subset_id: Uuid,
) -> Result<bool, PipelineRepoError> {
    let n = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM scenario_subsets WHERE scenario_id = $1 AND subset_id = $2",
    )
    .bind(scenario_id)
    .bind(subset_id)
    .fetch_one(executor)
    .await?;
    Ok(n > 0)
}

/// How many history rows one subset carries. The count proof reads this.
pub async fn count_subset_history(
    executor: impl sqlx::PgExecutor<'_>,
    subset_id: Uuid,
) -> Result<i64, PipelineRepoError> {
    let n = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM chronology_subset_history WHERE subset_id = $1",
    )
    .bind(subset_id)
    .fetch_one(executor)
    .await?;
    Ok(n)
}

// ─── The three by-id reads a subset needs and no other surface does ──────────
//
// The chronology's own readers all filter `deleted_at IS NULL` (see
// `chronology.rs`'s header: "reads never see a deleted row"). A subset read
// must see one — that is what a gap IS — so it cannot use them, and widening
// them would weaken a promise five other surfaces rely on. These three live
// here, beside the only caller that wants them, with the reason written down.

/// The events these ids name, in ANY state, deleted ones included.
///
/// Returns [`super::chronology_write::ChronologyEventStateRow`] — the row type
/// that carries `deleted_at` — because that column is the whole point of the
/// call. A missing id simply does not come back; the composer reports it as a
/// warning rather than silently shortening the story.
pub async fn events_any_state_by_ids(
    executor: impl sqlx::PgExecutor<'_>,
    event_ids: &[Uuid],
) -> Result<Vec<super::chronology_write::ChronologyEventStateRow>, PipelineRepoError> {
    if event_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, super::chronology_write::ChronologyEventStateRow>(
        "SELECT id, case_slug, event_date, date_precision, approximate, phase, title, \
                fact, attributes, created_by, created_at, updated_by, updated_at, deleted_at \
         FROM chronology_events WHERE id = ANY($1)",
    )
    .bind(event_ids)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// Every link on these events, in ONE query.
///
/// The per-event sibling exists (`chronology_links::list_links_for_event`) and
/// calling it in a loop is the N+1 a fifteen-event story would pay fifteen times.
pub async fn links_for_events(
    executor: impl sqlx::PgExecutor<'_>,
    event_ids: &[Uuid],
) -> Result<Vec<super::chronology::ChronologyLinkRow>, PipelineRepoError> {
    if event_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, super::chronology::ChronologyLinkRow>(
        "SELECT event_id, target_type, target_id, label, pinpoint, created_by, created_at \
         FROM chronology_event_links WHERE event_id = ANY($1) \
         ORDER BY event_id, target_type, target_id",
    )
    .bind(event_ids)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// `(event_id, live note count)` for these events, for the cards' badges.
///
/// Only events that HAVE notes come back; the composer treats a missing event as
/// zero, which is the same fact expressed with fewer rows.
pub async fn note_counts_for_events(
    executor: impl sqlx::PgExecutor<'_>,
    event_ids: &[Uuid],
) -> Result<Vec<(Uuid, i64)>, PipelineRepoError> {
    if event_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, (Uuid, i64)>(
        "SELECT event_id, COUNT(*) FROM chronology_event_notes \
         WHERE event_id = ANY($1) AND deleted_at IS NULL \
         GROUP BY event_id",
    )
    .bind(event_ids)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}
