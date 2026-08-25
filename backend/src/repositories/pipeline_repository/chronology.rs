//! Row types and phase/event reads for the case chronology.
//!
//! CASE_CHRONOLOGY_DESIGN_v2 §4. Every table here lives in `colossus_legal_v2`,
//! so every call takes the PIPELINE pool — `&state.pipeline_pool`, never
//! `state.pg_pool`. The siblings `chronology_links` (links, notes, history,
//! target resolution) and `chronology_write` (the seed's inserts) split off to
//! stay under the 300-line limit; the row types they share live here.
//!
//! ## Reads never see a deleted row
//!
//! Delete is soft (design R10), so `deleted_at IS NULL` is part of every read in
//! this module rather than something each caller remembers. A caller that one
//! day needs the deleted rows — the Undo line, in Phase C — will ask for them by
//! name through a function that says so.

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use super::PipelineRepoError;

/// One phase of the case, as `chronology_phases` stores it.
///
/// The four rows are seeded by migration `20260825105447` verbatim from the
/// retiring `timeline.json`. `description` is `Option` because the column is
/// nullable — design R14 renders it as a muted subtitle, and a phase without one
/// simply has no subtitle.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChronologyPhaseRow {
    pub id: String,
    pub label: String,
    pub date_range: String,
    pub color: String,
    pub description: Option<String>,
    pub sort_order: i32,
}

/// One dated fact, as `chronology_events` stores it.
///
/// ## Rust Learning: `serde_json::Value` as a sqlx column type
///
/// sqlx decodes a Postgres `jsonb` column straight into `serde_json::Value` with
/// the `json` feature on. Keeping it as a `Value` — rather than a typed struct —
/// IS the change rule (design R4): a key this build has never heard of arrives,
/// survives a read, and survives the next write, instead of failing to
/// deserialize and taking the row's whole event down with it.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChronologyEventRow {
    pub id: Uuid,
    pub case_id: String,
    pub event_date: NaiveDate,
    pub date_precision: String,
    pub approximate: bool,
    pub title: String,
    pub fact: Option<String>,
    pub attributes: serde_json::Value,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_by: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// One link from an event to its evidence, as `chronology_event_links` stores it.
///
/// There is no surrogate id: the natural key is `(event_id, target_type,
/// target_id)` — an event cannot link the same target twice — so nothing had to
/// be invented to address a row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChronologyLinkRow {
    pub event_id: Uuid,
    pub target_type: String,
    pub target_id: String,
    pub label: Option<String>,
    /// Page, paragraph, Q-number, line. `None` is MEANINGFUL — the surface marks
    /// it "no pinpoint" so unpinpointed events read as the to-scan list (R9).
    pub pinpoint: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// One attributed note on an event (design R8).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChronologyNoteRow {
    pub id: Uuid,
    pub event_id: Uuid,
    pub note: String,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// One append-only history entry for an event.
///
/// Nothing writes these in Phase A — the write endpoints are Phase C. The type
/// and its read exist now because the event-detail endpoint returns history, and
/// an endpoint that cannot read an empty list is a 500 waiting to happen.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChronologyHistoryRow {
    pub id: Uuid,
    pub event_id: Uuid,
    pub action: String,
    pub snapshot: serde_json::Value,
    pub changed_by: Option<String>,
    pub changed_at: DateTime<Utc>,
}

/// The projection every event read shares, so two queries cannot disagree about
/// the column set `ChronologyEventRow` expects.
///
/// `deleted_at` is deliberately absent: no read in this module returns a deleted
/// row, so no caller is handed a column it must remember to check.
const EVENT_COLUMNS: &str = "id, case_id, event_date, date_precision, approximate, \
     title, fact, attributes, created_by, created_at, updated_by, updated_at";

/// Every phase, in the case's chronological order.
///
/// Ordered by the stored `sort_order` rather than by id or label: the order is
/// the order the case happened in, and it is data, not alphabet.
pub async fn list_phases(
    executor: impl sqlx::PgExecutor<'_>,
) -> Result<Vec<ChronologyPhaseRow>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ChronologyPhaseRow>(
        "SELECT id, label, date_range, color, description, sort_order \
         FROM chronology_phases ORDER BY sort_order",
    )
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// Every live event for one case, oldest first.
///
/// `(event_date, id)` is the stored index's order, so two reads of unchanged
/// data can never swap two events that share a date.
pub async fn list_events(
    executor: impl sqlx::PgExecutor<'_>,
    case_id: &str,
) -> Result<Vec<ChronologyEventRow>, PipelineRepoError> {
    let sql = format!(
        "SELECT {EVENT_COLUMNS} FROM chronology_events \
         WHERE case_id = $1 AND deleted_at IS NULL \
         ORDER BY event_date, id"
    );
    let rows = sqlx::query_as::<_, ChronologyEventRow>(&sql)
        .bind(case_id)
        .fetch_all(executor)
        .await?;
    Ok(rows)
}

/// One live event by id, or `None` when there is no such live event.
///
/// ## Rust Learning: `Option` here, `NotFound` at the handler
///
/// The repository reports absence as `Ok(None)` — "the query ran, there was no
/// row" — and leaves the decision of what absence MEANS to the caller. The
/// handler turns it into a 404 with a message. A repository that returned
/// `Err(NotFound)` would force every caller that legitimately expects nothing to
/// pattern-match an error to learn a fact.
pub async fn get_event(
    executor: impl sqlx::PgExecutor<'_>,
    id: Uuid,
) -> Result<Option<ChronologyEventRow>, PipelineRepoError> {
    let sql = format!(
        "SELECT {EVENT_COLUMNS} FROM chronology_events \
         WHERE id = $1 AND deleted_at IS NULL"
    );
    let row = sqlx::query_as::<_, ChronologyEventRow>(&sql)
        .bind(id)
        .fetch_optional(executor)
        .await?;
    Ok(row)
}

/// How many live events one case holds.
///
/// Used by the seed one-shot's count proof, which must be able to state the
/// before and after without pulling 22 rows across the wire to length them.
pub async fn count_events(
    executor: impl sqlx::PgExecutor<'_>,
    case_id: &str,
) -> Result<i64, PipelineRepoError> {
    let n = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM chronology_events WHERE case_id = $1 AND deleted_at IS NULL",
    )
    .bind(case_id)
    .fetch_one(executor)
    .await?;
    Ok(n)
}

/// How many phase rows exist. The seed's verification asserts this is four.
pub async fn count_phases(executor: impl sqlx::PgExecutor<'_>) -> Result<i64, PipelineRepoError> {
    let n = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM chronology_phases")
        .fetch_one(executor)
        .await?;
    Ok(n)
}
