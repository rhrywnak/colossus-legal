//! Repository for `scenario_human_facts` in the `colossus_legal_v2` pipeline
//! database — C4, knowledge that is in no document (v2 §2, §8).
//!
//! ## What makes these rows different from every other fact in the system
//!
//! They carry no citation. Every record item is anchored (document, page,
//! verbatim quote, speaker) so it can be defended; a human fact has an AUTHOR in
//! its place. That is not a gap to be filled later — "switching counsel meant
//! emailing attachments" is true, load-bearing, and written down nowhere. The
//! surface labels these rows so a reader is never in doubt which kind they are
//! looking at.
//!
//! ## The §8 invariant, and where it is enforced
//!
//! "Re-gathering never edits human content." This module is the ONLY writer of
//! `scenario_human_facts`, and no scan, gather or merge path calls it. That is
//! asserted, not assumed: `write_path_invariants` scans the source tree and pins
//! the allowlist of tables the scan/merge code may write, so a future writer
//! reaching into human content fails the build.
//!
//! The other half of §8 — "editing human content never triggers re-gathering" —
//! is a property of the API layer: none of the augmentation handlers calls the
//! gather path. Also asserted there.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

/// A row from `scenario_human_facts`.
///
/// `date_type` is kept as its raw token rather than the typed
/// [`crate::domain::human_authored::DateType`], matching how
/// `ScenarioFactRefRecord` keeps `status` raw: the enum is a parse type used at
/// the call site, and decoding here would make every read fallible for a value
/// most readers only pass through.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ScenarioHumanFactRecord {
    pub id: Uuid,
    pub scenario_id: Uuid,
    pub text: String,
    pub occurred_on: Option<NaiveDate>,
    pub date_type: Option<String>,
    /// Plain names, not entity ids — canonical identity is task B0.
    pub person_refs: Option<Vec<String>>,
    pub authored_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Shared SELECT projection, so the `FromRow` column set cannot drift between
/// query sites (the house anti-drift discipline).
// CONST: column projection locked to the `scenario_human_facts` schema — a
// structural schema-coupling invariant, not a deployment value.
const HUMAN_FACT_COLUMNS: &str = "id, scenario_id, text, occurred_on, date_type, \
     person_refs, authored_by, created_at, updated_at";

/// Everything needed to write one human fact.
///
/// A params struct rather than eight positional arguments: two adjacent
/// `Option<String>` fields (`date_type`) and a `Vec<String>` would be swappable
/// at a call site with no compile error, and the author field must never be
/// confused with the text.
#[derive(Debug)]
pub struct HumanFactWrite<'a> {
    pub scenario_id: Uuid,
    pub text: &'a str,
    pub occurred_on: Option<NaiveDate>,
    /// Only meaningful with a date — the table CHECKs the pairing.
    pub date_type: Option<&'a str>,
    pub person_refs: &'a [String],
    pub authored_by: &'a str,
    /// Passed in rather than read from the clock here, so the row's timestamp
    /// matches the log line and tests stay deterministic.
    pub written_at: DateTime<Utc>,
}

// CONST: the insert. Held as a `const` so a SQL-shape test can pin it without a
// live database (house pattern).
const INSERT_HUMAN_FACT_SQL: &str = r#"INSERT INTO scenario_human_facts
        (scenario_id, text, occurred_on, date_type, person_refs,
         authored_by, created_at, updated_at)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
    RETURNING id"#;

/// Write one human fact and return its id.
///
/// `created_at` and `updated_at` both bind `$7` on insert: a row that has never
/// been edited has the two equal, which is what lets a reader tell an edited fact
/// from a fresh one without a separate flag.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the insert fails — notably the blank-text
/// CHECK, or the date-type-needs-a-date CHECK if a caller sends a type with no
/// date.
pub async fn insert_human_fact(
    executor: impl sqlx::PgExecutor<'_>,
    fact: &HumanFactWrite<'_>,
) -> Result<Uuid, PipelineRepoError> {
    let id = sqlx::query_scalar::<_, Uuid>(INSERT_HUMAN_FACT_SQL)
        .bind(fact.scenario_id)
        .bind(fact.text)
        .bind(fact.occurred_on)
        .bind(fact.date_type)
        .bind(fact.person_refs)
        .bind(fact.authored_by)
        .bind(fact.written_at)
        .fetch_one(executor)
        .await?;
    Ok(id)
}

/// List a scenario's human facts, oldest first.
///
/// Oldest-first because these accumulate as a working record and a human reads
/// them in the order they were learned — the reverse of a feed.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn list_human_facts_for_scenario(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<ScenarioHumanFactRecord>, PipelineRepoError> {
    let sql = format!(
        "SELECT {HUMAN_FACT_COLUMNS} FROM scenario_human_facts \
         WHERE scenario_id = $1 ORDER BY created_at, id"
    );
    let rows = sqlx::query_as::<_, ScenarioHumanFactRecord>(&sql)
        .bind(scenario_id)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

// CONST: the delete. Held as a `const` for the same reason as the insert — so a
// test can pin the SCENARIO FENCE against the production statement rather than
// against a copy of it in the test file, which would assert nothing.
const DELETE_HUMAN_FACT_SQL: &str =
    "DELETE FROM scenario_human_facts WHERE id = $1 AND scenario_id = $2";

/// Delete one human fact. Returns how many rows went (0 = no such fact here).
///
/// Scoped by `scenario_id` as well as `id`: the id alone would let a caller
/// delete another scenario's fact by guessing a UUID, and the scenario is the
/// fence every other route in this family already applies.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the delete fails.
pub async fn delete_human_fact(
    executor: impl sqlx::PgExecutor<'_>,
    scenario_id: Uuid,
    fact_id: Uuid,
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(DELETE_HUMAN_FACT_SQL)
        .bind(fact_id)
        .bind(scenario_id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
#[path = "scenario_human_facts_tests.rs"]
mod tests;
