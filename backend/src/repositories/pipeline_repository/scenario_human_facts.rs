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
    /// `fact` | `watch_list` (task 1.5). Kept as its raw token for the same
    /// reason as `date_type`: the enum is a parse type used at the call site.
    pub kind: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// The statement this judgment is ABOUT, for the anchor kinds (task 2.11).
    /// `None` on a fact or a watch-list note, which are about nothing but
    /// themselves.
    pub anchor_graph_node_id: Option<String>,
    /// For an `answer_pairing`, the record item that answers the anchor. `None`
    /// on every other kind — and never inferred, because the machine does not
    /// assert rebuttal.
    pub answers_graph_node_id: Option<String>,
}

/// Shared SELECT projection, so the `FromRow` column set cannot drift between
/// query sites (the house anti-drift discipline).
// CONST: column projection locked to the `scenario_human_facts` schema — a
// structural schema-coupling invariant, not a deployment value.
// Task 2.11 added the two anchor columns. THIS CONST IS THE DRIFT POINT: every
// reader goes through it, so a column added to the migration and the record but
// forgotten here is a runtime decode failure on every read at once — the same
// seam `SCENARIO_FACT_REF_COLUMNS` carries, and pinned the same way.
const HUMAN_FACT_COLUMNS: &str = "id, scenario_id, text, occurred_on, date_type, \
     person_refs, authored_by, kind, created_at, updated_at, \
     anchor_graph_node_id, answers_graph_node_id";

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
    /// `fact` or `watch_list` — both human-authored, both equally protected by
    /// the §8 invariants, which is why they share this write path.
    pub kind: &'a str,
    /// Passed in rather than read from the clock here, so the row's timestamp
    /// matches the log line and tests stay deterministic.
    pub written_at: DateTime<Utc>,
}

/// Everything needed to record one ANCHORED judgment (task 2.11).
///
/// Separate from [`HumanFactWrite`] because these rows are a different shape,
/// not a variation on it: they carry no prose, no date and no people — only a
/// pointer at the statement being judged. Sharing one params struct would mean
/// six fields that must be empty for one kind and required for the other, which
/// is the sort of "valid only in combination" type the rest of this codebase
/// splits apart on sight.
///
/// The §8 guards are NOT duplicated: both writers are reached through the same
/// handler fence (edit gate, scenario-in-case check), which is what "one write
/// path" protects.
#[derive(Debug)]
pub struct AnchoredJudgmentWrite<'a> {
    pub scenario_id: Uuid,
    /// The statement being judged — a graph node id (§12), never an internal id.
    pub anchor_graph_node_id: &'a str,
    /// For an answer pairing, the record item that answers it. `None` for an
    /// accusation instance, which judges the anchor alone.
    pub answers_graph_node_id: Option<&'a str>,
    pub authored_by: &'a str,
    /// `accusation_instance` or `answer_pairing`.
    pub kind: &'a str,
    pub written_at: DateTime<Utc>,
}

// CONST: the insert. Held as a `const` so a SQL-shape test can pin it without a
// live database (house pattern).
const INSERT_HUMAN_FACT_SQL: &str = r#"INSERT INTO scenario_human_facts
        (scenario_id, text, occurred_on, date_type, person_refs,
         authored_by, kind, created_at, updated_at)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
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
        .bind(fact.kind)
        .bind(fact.written_at)
        .fetch_one(executor)
        .await?;
    Ok(id)
}

// CONST: the anchored upsert. Held as a `const` so a SQL-shape test can pin it
// without a live database, exactly like the insert above.
//
// `text` binds the empty string: an anchored row says nothing, and the table's
// CHECK is "says something OR points at something", so an empty text with a
// non-null anchor is the legal shape rather than a workaround.
const UPSERT_ANCHORED_SQL: &str = r#"INSERT INTO scenario_human_facts
        (scenario_id, text, anchor_graph_node_id, answers_graph_node_id,
         authored_by, kind, created_at, updated_at)
    VALUES ($1, '', $2, $3, $4, $5, $6, $6)
    ON CONFLICT (scenario_id, anchor_graph_node_id) WHERE kind = 'answer_pairing'
    DO UPDATE SET answers_graph_node_id = EXCLUDED.answers_graph_node_id,
                  authored_by           = EXCLUDED.authored_by,
                  updated_at            = EXCLUDED.updated_at
    RETURNING id"#;

/// Record an accusation instance, or pair an answer to one — re-pairing in place.
///
/// ## Why this is an UPSERT and what the conflict target means
///
/// The migration puts a PARTIAL unique index on `(scenario_id,
/// anchor_graph_node_id) WHERE kind = 'answer_pairing'`: one answer per
/// accusation. Re-pairing must therefore REPLACE, not accumulate — two stored
/// answers to one accusation would leave the rehearsal page choosing between
/// them with no basis for choosing, which is exactly the kind of silent decision
/// the honest-gap law forbids.
///
/// `created_at` is deliberately NOT touched on conflict: it records when this
/// accusation first got an answer, and re-pairing is an edit to that answer, not
/// a new judgment. `authored_by` IS updated — the person who last decided what
/// answers this is the person answerable for it.
///
/// An `accusation_instance` never conflicts on this target (the predicate
/// excludes it) and is protected instead by its own partial index, so a
/// duplicate mark surfaces as a unique violation rather than a second row. The
/// caller treats that as "already marked", which it is.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the statement fails — including the unique
/// violation above, and the CHECK that an answer must carry an anchor.
pub async fn upsert_anchored_judgment(
    executor: impl sqlx::PgExecutor<'_>,
    judgment: &AnchoredJudgmentWrite<'_>,
) -> Result<Uuid, PipelineRepoError> {
    let id = sqlx::query_scalar::<_, Uuid>(UPSERT_ANCHORED_SQL)
        .bind(judgment.scenario_id)
        .bind(judgment.anchor_graph_node_id)
        .bind(judgment.answers_graph_node_id)
        .bind(judgment.authored_by)
        .bind(judgment.kind)
        .bind(judgment.written_at)
        .fetch_one(executor)
        .await?;
    Ok(id)
}

// CONST: marking one accusation instance. Its own statement rather than a
// parameter on the upsert above, because the two row shapes need two different
// conflict targets and a statement can name only one — the sanctioned reading of
// "one write path" (2026-08-06): one module, one fence, two statements.
//
// `DO NOTHING` rather than `DO UPDATE`: re-marking something already marked is
// the SAME judgment, so the first author stands. Moving `authored_by` on a stale
// double-click would quietly reassign a decision somebody else made.
const INSERT_INSTANCE_SQL: &str = r#"INSERT INTO scenario_human_facts
        (scenario_id, text, anchor_graph_node_id, authored_by, kind,
         created_at, updated_at)
    VALUES ($1, '', $2, $3, $4, $5, $5)
    ON CONFLICT (scenario_id, anchor_graph_node_id) WHERE kind = 'accusation_instance'
    DO NOTHING
    RETURNING id"#;

/// Mark a statement as an accusation instance, idempotently.
///
/// Returns `Some(id)` when a marking was recorded and `None` when this statement
/// was ALREADY marked. The caller needs that distinction — the second means the
/// human's page was stale, which is worth a log line even though their intent is
/// satisfied either way.
///
/// ## Rust Learning: `fetch_optional` with `ON CONFLICT DO NOTHING RETURNING`
///
/// Postgres returns NO ROW when the conflict clause does nothing, so
/// `fetch_one` would fail with `RowNotFound` for the ordinary "already marked"
/// case and force the caller to classify a database error. `fetch_optional` turns
/// the same outcome into an `Option`, which is a typed answer rather than an
/// error to interpret.
///
/// ## Why this is not a SELECT followed by an INSERT
///
/// Two statements have a race between them, and the race is the COMMON case here:
/// a human double-clicking. The partial unique index decides, atomically, and this
/// reads its verdict.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the statement fails to execute.
pub async fn insert_accusation_instance(
    executor: impl sqlx::PgExecutor<'_>,
    scenario_id: Uuid,
    anchor_graph_node_id: &str,
    authored_by: &str,
    written_at: DateTime<Utc>,
) -> Result<Option<Uuid>, PipelineRepoError> {
    let id = sqlx::query_scalar::<_, Uuid>(INSERT_INSTANCE_SQL)
        .bind(scenario_id)
        .bind(anchor_graph_node_id)
        .bind(authored_by)
        .bind(crate::domain::human_authored::HumanFactKind::AccusationInstance.code())
        .bind(written_at)
        .fetch_optional(executor)
        .await?;
    Ok(id)
}

// CONST: removing one anchored judgment, by what it is ABOUT rather than by id.
//
// The caller holds the anchor — it is what the human clicked — and not a row id
// it would have to look up first. Keyed on kind as well so un-marking an
// instance cannot delete the pairing that hangs off the same anchor.
const DELETE_ANCHORED_SQL: &str = r#"DELETE FROM scenario_human_facts
    WHERE scenario_id = $1 AND anchor_graph_node_id = $2 AND kind = $3"#;

/// Withdraw an accusation instance or an answer pairing.
///
/// Returns the number of rows removed, so the caller can tell "withdrawn" from
/// "there was nothing there" — a distinction the human needs, because the second
/// means their page is stale.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the statement fails to execute.
pub async fn delete_anchored_judgment(
    executor: impl sqlx::PgExecutor<'_>,
    scenario_id: Uuid,
    anchor_graph_node_id: &str,
    kind: &str,
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(DELETE_ANCHORED_SQL)
        .bind(scenario_id)
        .bind(anchor_graph_node_id)
        .bind(kind)
        .execute(executor)
        .await?;
    Ok(result.rows_affected())
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
