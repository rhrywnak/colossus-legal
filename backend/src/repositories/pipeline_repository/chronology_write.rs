//! The chronology's writes — used ONLY by the seed one-shot in Phase A.
//!
//! There is no HTTP path to either function today. The write endpoints are
//! Phase C, and keeping the inserts in their own module makes that boundary
//! something a reader can see rather than something a comment claims: if this
//! module ever appears in an `api::` import list before Phase C, the review
//! question asks itself.
//!
//! Both functions take `impl sqlx::PgExecutor<'_>` so the caller can hand them a
//! `&mut PgConnection` inside a transaction — the seed writes all 22 events and
//! their links in ONE transaction, so a failure half way leaves no partial
//! chronology behind.

use chrono::NaiveDate;
use uuid::Uuid;

use super::PipelineRepoError;

/// Everything one new event needs.
///
/// ## Rust Learning: a parameter struct instead of nine positional arguments
///
/// `insert_event(pool, "awad", date, "day", false, "title", None, attrs, "roman")`
/// has three `&str` in a row and two `Option`s; swapping any two would compile
/// and write the wrong row. Naming each field at the call site makes that class
/// of mistake impossible, and it keeps the function under the argument count
/// where a reader stops tracking positions.
#[derive(Debug, Clone)]
pub struct NewChronologyEvent<'a> {
    pub case_slug: &'a str,
    pub event_date: NaiveDate,
    pub date_precision: &'a str,
    pub approximate: bool,
    /// The phase slug. A real column with a foreign key, so a value that is not
    /// one of the four is refused by the database, not merely by a reviewer.
    pub phase: &'a str,
    pub title: &'a str,
    pub fact: Option<&'a str>,
    pub attributes: &'a serde_json::Value,
    pub created_by: &'a str,
}

/// Insert one event and return the id the database generated.
///
/// No `ON CONFLICT`: an event has no natural key — two real events can share a
/// date and a title — so there is nothing to conflict ON, and a re-run that
/// silently merged rows would be worse than one that visibly doubles them. The
/// one-shot's own guard is that it refuses to seed a case that already holds
/// events.
pub async fn insert_event(
    executor: impl sqlx::PgExecutor<'_>,
    event: &NewChronologyEvent<'_>,
) -> Result<Uuid, PipelineRepoError> {
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO chronology_events \
             (case_slug, event_date, date_precision, approximate, phase, title, \
              fact, attributes, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
    )
    .bind(event.case_slug)
    .bind(event.event_date)
    .bind(event.date_precision)
    .bind(event.approximate)
    .bind(event.phase)
    .bind(event.title)
    .bind(event.fact)
    .bind(event.attributes)
    .bind(event.created_by)
    .fetch_one(executor)
    .await?;
    Ok(id)
}

/// Everything one new link needs.
#[derive(Debug, Clone)]
pub struct NewChronologyLink<'a> {
    pub event_id: Uuid,
    pub target_type: &'a str,
    pub target_id: &'a str,
    pub label: Option<&'a str>,
    /// `None` is the honest value for every seeded link: the legacy JSON carried
    /// no pinpoints, and inventing one would be exactly the fabrication the
    /// date-precision track exists to prevent.
    pub pinpoint: Option<&'a str>,
    pub created_by: &'a str,
}

/// Insert one link.
///
/// `ON CONFLICT DO NOTHING` on the natural key: linking the same target to the
/// same event twice is not an error, it is the same fact stated twice, and the
/// second statement should be a no-op rather than a constraint violation that
/// aborts a 22-event transaction.
pub async fn insert_link(
    executor: impl sqlx::PgExecutor<'_>,
    link: &NewChronologyLink<'_>,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "INSERT INTO chronology_event_links \
             (event_id, target_type, target_id, label, pinpoint, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (event_id, target_type, target_id) DO NOTHING",
    )
    .bind(link.event_id)
    .bind(link.target_type)
    .bind(link.target_id)
    .bind(link.label)
    .bind(link.pinpoint)
    .bind(link.created_by)
    .execute(executor)
    .await?;
    Ok(())
}

/// How many link rows one case's live events carry. Part of the seed's proof.
pub async fn count_links(
    executor: impl sqlx::PgExecutor<'_>,
    case_slug: &str,
) -> Result<i64, PipelineRepoError> {
    let n = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM chronology_event_links l \
         JOIN chronology_events e ON e.id = l.event_id \
         WHERE e.case_slug = $1 AND e.deleted_at IS NULL",
    )
    .bind(case_slug)
    .fetch_one(executor)
    .await?;
    Ok(n)
}
