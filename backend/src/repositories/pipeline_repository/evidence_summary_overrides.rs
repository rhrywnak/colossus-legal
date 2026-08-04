//! Repository for `evidence_summary_overrides` in the `colossus_legal_v2`
//! pipeline database — a human's correction of a machine-written interrogatory
//! question (task 1.7F Part B).
//!
//! ## The machine's words are in a different database, and that is the safety
//!
//! `Evidence.question` lives in Neo4j. A row here lives in Postgres beside it and
//! is applied at CARD COMPOSITION time, never merged back. So ruling R2 — "the
//! machine original is untouchable" — is not a discipline this module has to
//! keep; it is a property of the storage boundary. Nothing in this file can reach
//! the graph, and `revert` is a DELETE rather than a copy-back for the same
//! reason: there is nothing to copy back to.
//!
//! ## Domain note: CASE-WIDE by key (ruling R1)
//!
//! The primary key is the evidence node ALONE. Correct a question while working
//! scenario S-2 and it reads corrected in S-4, deliberately: the same quote is the
//! same quote, and a per-scenario override would let one C-code mean two things
//! depending on which page a reader is standing on. If a future change adds a
//! `scenario_id` column here, it has re-created exactly that defect.
//!
//! ## v2 §8: human-authored content
//!
//! This module is the ONLY writer of the table, and no scan, gather or merge path
//! calls it. That is asserted rather than assumed — the table is listed in
//! `HUMAN_AUTHORED_TABLES`, and the two scan-path invariant tests in
//! `scenario_human_facts_tests` fail the build if a scan path so much as names it.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::PipelineRepoError;

/// A row from `evidence_summary_overrides`.
///
/// ## Rust learning: `sqlx::FromRow` and why the types are not negotiable
///
/// The derive maps columns to fields BY NAME, and each field's type must be one
/// sqlx can decode from that column's SQL type. `TEXT` → [`String`] and
/// `TIMESTAMPTZ` → [`DateTime<Utc>`] are the two pairings here, both of which the
/// house already uses (`scenario_human_facts`). Nothing in this table is
/// `NUMERIC`: beta.364 died at boot decoding one into an `Option<f64>` with no
/// `rust_decimal` in the tree, and the way that stays fixed is by not introducing
/// the type at all.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EvidenceSummaryOverrideRecord {
    pub graph_node_id: String,
    pub summary_text: String,
    pub authored_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Shared SELECT projection, so the `FromRow` column set cannot drift between
/// query sites (the house anti-drift discipline).
// CONST: column projection locked to the `evidence_summary_overrides` schema — a
// structural schema-coupling invariant, not a deployment value.
const OVERRIDE_COLUMNS: &str = "graph_node_id, summary_text, authored_by, created_at, updated_at";

/// Everything needed to write one override.
///
/// A params struct rather than four positional arguments: `summary_text` and
/// `authored_by` are both `&str` and adjacent, so a call site could swap them
/// with no compile error and record the human's correction as its author.
#[derive(Debug)]
pub struct SummaryOverrideWrite<'a> {
    pub graph_node_id: &'a str,
    pub summary_text: &'a str,
    pub authored_by: &'a str,
    /// Passed in rather than read from the clock here, so the row's timestamp
    /// matches the log line and tests stay deterministic.
    pub written_at: DateTime<Utc>,
}

// CONST: the upsert. Held as a `const` so a SQL-shape test can pin it without a
// live database (house pattern).
//
// `created_at` is written on insert and deliberately NOT touched on conflict: a
// correction made in June and re-edited today keeps its original date, so
// "corrected once, long ago" stays distinguishable from "corrected this morning".
// `updated_at` is what the badge shows.
const UPSERT_OVERRIDE_SQL: &str = r#"INSERT INTO evidence_summary_overrides
        (graph_node_id, summary_text, authored_by, created_at, updated_at)
    VALUES ($1, $2, $3, $4, $4)
    ON CONFLICT (graph_node_id) DO UPDATE SET
        summary_text = EXCLUDED.summary_text,
        authored_by  = EXCLUDED.authored_by,
        updated_at   = EXCLUDED.updated_at"#;

/// Write (or replace) one evidence summary override.
///
/// Upsert rather than insert-or-error: correcting a question twice is ordinary
/// work, not a conflict, and the second correction is simply the current one.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the write fails — notably the blank-text
/// CHECK, which is the database's backstop behind the handler's own trim.
pub async fn upsert_summary_override(
    executor: impl sqlx::PgExecutor<'_>,
    write: &SummaryOverrideWrite<'_>,
) -> Result<(), PipelineRepoError> {
    sqlx::query(UPSERT_OVERRIDE_SQL)
        .bind(write.graph_node_id)
        .bind(write.summary_text)
        .bind(write.authored_by)
        .bind(write.written_at)
        .execute(executor)
        .await?;
    Ok(())
}

/// Remove one override — the "revert to the system text" path (ruling R2).
///
/// Returns whether a row was actually deleted, so the caller can tell "reverted"
/// from "there was nothing to revert" and say so. Collapsing the two would let a
/// revert on an un-overridden item report success it did not perform.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the delete fails.
pub async fn delete_summary_override(
    executor: impl sqlx::PgExecutor<'_>,
    graph_node_id: &str,
) -> Result<bool, PipelineRepoError> {
    let result = sqlx::query("DELETE FROM evidence_summary_overrides WHERE graph_node_id = $1")
        .bind(graph_node_id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Every override for a set of evidence nodes, in ONE query.
///
/// ## Why a batch read and not one lookup per card
///
/// The cards endpoint composes 148 cards per request. A per-card query would be
/// 148 round trips to answer a question one `= ANY($1)` answers, which is the
/// N+1 the sibling reads here (`list_fact_refs_for_scenario`,
/// `list_candidate_ordinals`) already avoid by the same shape.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn list_summary_overrides(
    pool: &PgPool,
    graph_node_ids: &[String],
) -> Result<Vec<EvidenceSummaryOverrideRecord>, PipelineRepoError> {
    // An empty pool is a real state (a scenario nobody has scanned), and
    // `= ANY('{}')` is a round trip that can only return nothing.
    if graph_node_ids.is_empty() {
        return Ok(Vec::new());
    }
    let sql = format!(
        "SELECT {OVERRIDE_COLUMNS} FROM evidence_summary_overrides \
         WHERE graph_node_id = ANY($1)"
    );
    let rows = sqlx::query_as::<_, EvidenceSummaryOverrideRecord>(&sql)
        .bind(graph_node_ids)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

#[cfg(test)]
#[path = "evidence_summary_overrides_tests.rs"]
mod tests;
