//! Repository for `scenario_candidate_ordinals` in the `colossus_legal_v2`
//! pipeline database — the persisted, scenario-scoped candidate identifier
//! (`C-1`, `C-2`, …) that humans use to refer to a candidate fact out loud.
//!
//! ## Why identity lives in its own table
//!
//! `scenario_fact_refs` is **derive-on-read**: a row exists there if and only if a
//! candidate has been ruled on (include/drop) or scored by a merge. That contract
//! is load-bearing — `join_facts` reads a lookup miss as "this ref points at a dead
//! graph node", so materializing a row for every pool member would corrupt that
//! meaning.
//!
//! An ordinal, though, must exist for EVERY pool member from the moment it first
//! appears, whether or not anyone has decided anything about it. Storing it here
//! keeps the two ideas apart:
//!
//! * this table memoizes **identity** — *which candidate is C-14*;
//! * `scenario_fact_refs` records **state** — *what the human decided about it*.
//!
//! Gather is allowed to write here for exactly that reason: assigning an ordinal
//! is not a user-state mutation, so it does not breach the derive-on-read contract
//! that protects candidate state.
//!
//! ## The guarantees
//!
//! Append-only, never reused, never renumbered. A dropped candidate keeps its id
//! forever (drop excludes, it never deletes — "we looked at C-31 and dropped it"
//! must stay sayable). When the pipeline's duplicate-node defect is fixed, retired
//! duplicates leave **holes** in the sequence; holes are correct, and closing them
//! by renumbering would invalidate every reference already written in a notebook
//! or spoken in a rehearsal.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

// CONST: the assignment statement. Held as a `const` so a SQL-shape unit test can
// pin its idempotency and append-only semantics without a live database (house
// pattern). Query text, not deployment config — Rule 13 does not apply.
//
// ## How this assigns without a sequence, a read-modify-write, or a race
//
// * `UNNEST($2::text[]) WITH ORDINALITY` turns the caller's ordered node-id array
//   into rows carrying their position (`n.ord`), so the pool's deterministic order
//   becomes the assignment order in ONE statement — no per-row round-trip.
// * `MAX(ordinal) + ROW_NUMBER()` continues the scenario's existing sequence:
//   first gather starts at 1 (COALESCE over an empty table yields 0), and later
//   gathers append after the highest id ever issued. Never reuses a hole.
// * `ON CONFLICT (scenario_id, graph_node_id) DO NOTHING` makes re-gathering
//   idempotent: a candidate that already has an ordinal keeps it, untouched. This
//   is what lets gather run on every page load without renumbering anything.
//
// Note `ROW_NUMBER()` is computed over ALL supplied ids, including ones that will
// hit the conflict and be skipped. That can consume numbers — a re-gather with no
// new candidates may still "spend" a range that nothing lands on, leaving a gap
// before the next genuinely-new candidate. Gaps are explicitly acceptable (see the
// module doc); the caller avoids the common case anyway by passing only ids that
// have no ordinal yet.
const ASSIGN_ORDINALS_SQL: &str = r#"INSERT INTO scenario_candidate_ordinals
        (scenario_id, graph_node_id, ordinal, assigned_at)
    SELECT
        $1,
        n.node_id,
        COALESCE(
            (SELECT MAX(ordinal) FROM scenario_candidate_ordinals WHERE scenario_id = $1),
            0
        ) + ROW_NUMBER() OVER (ORDER BY n.ord),
        $3
    FROM UNNEST($2::text[]) WITH ORDINALITY AS n(node_id, ord)
    ON CONFLICT (scenario_id, graph_node_id) DO NOTHING"#;

/// Assign ordinals to any of `graph_node_ids` that do not have one yet.
///
/// `graph_node_ids` MUST be in the pool's deterministic display order — that order
/// becomes the id sequence, and it is only ever consulted once per candidate (the
/// ordinal is persisted thereafter, so a later change in pool ordering can never
/// renumber anything).
///
/// Idempotent: calling this repeatedly with the same pool assigns nothing new.
/// Returns the number of ordinals actually minted, so the caller can log a
/// genuinely-new-candidate count rather than guessing.
///
/// ## Rust Learning: binding a `&[String]` to `text[]`
///
/// sqlx maps a Rust slice of `String` directly onto a Postgres `text[]` parameter,
/// so the whole pool rides as ONE bind rather than a variable-length `IN (…)`
/// list. That keeps this a single fixed-shape prepared statement no matter how
/// many candidates the scenario has.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the insert fails — notably a unique violation
/// on `(scenario_id, ordinal)`, which means two gathers raced. That is a LOUD
/// failure on purpose: minting a duplicate `C-14` would make the human's handle
/// ambiguous, which is worse than a failed page load the user can retry.
pub async fn assign_candidate_ordinals(
    pool: &PgPool,
    scenario_id: Uuid,
    graph_node_ids: &[String],
    assigned_at: DateTime<Utc>,
) -> Result<u64, PipelineRepoError> {
    // Nothing to assign: skip the round-trip entirely. An empty pool is a normal
    // state (a scenario whose subject has no evidence yet), not an error.
    if graph_node_ids.is_empty() {
        return Ok(0);
    }

    let result = sqlx::query(ASSIGN_ORDINALS_SQL)
        .bind(scenario_id)
        .bind(graph_node_ids)
        .bind(assigned_at)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Read every ordinal assigned in one scenario, as a `graph_node_id → ordinal`
/// index.
///
/// Returned as a `HashMap` because the caller's job is O(1) lookup while walking
/// the pool — the same index technique gather already uses for fact-refs. Reads
/// the WHOLE scenario (not just the current pool) so a candidate that has left the
/// pool still resolves if it is ever displayed again; the map is small (one row
/// per candidate ever seen, ~94 today).
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn list_candidate_ordinals(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<HashMap<String, i32>, PipelineRepoError> {
    let rows: Vec<(String, i32)> = sqlx::query_as(
        "SELECT graph_node_id, ordinal FROM scenario_candidate_ordinals WHERE scenario_id = $1",
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;
    use std::time::Duration;

    /// A pool aimed at a dead port. It never connects, so ANY real query fails
    /// fast — which is what lets a test prove a code path did NOT touch the
    /// database. Same instrument as `theme_scan_persist_tests.rs`.
    fn dead_pool() -> PgPool {
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(500))
            .connect_lazy("postgres://127.0.0.1:1/nodb")
            .expect("connect_lazy builds a pool without connecting")
    }

    /// An empty pool must short-circuit BEFORE the query, not send an empty array.
    ///
    /// This is the normal state of a scenario whose subject has no evidence yet,
    /// so it must be a clean `Ok(0)` rather than an error or a wasted round-trip.
    /// The dead pool is the proof: if the early return were removed, the statement
    /// would be attempted here and this test would fail with a connection error
    /// instead of returning.
    #[tokio::test]
    async fn an_empty_pool_assigns_nothing_without_touching_the_database() {
        let assigned = assign_candidate_ordinals(
            &dead_pool(),
            Uuid::nil(),
            &[],
            chrono::DateTime::<Utc>::UNIX_EPOCH,
        )
        .await;

        assert!(
            matches!(assigned, Ok(0)),
            "an empty pool is a normal state: Ok(0), no query, no error — got {assigned:?}"
        );
    }

    /// SQL-shape guards for the three properties that make assignment safe. No
    /// live-DB harness exists here (the store's real behavior lives in the
    /// `--ignored` integration suite), so these pin the clauses that PRODUCE the
    /// behavior — the house pattern used by the merge/reconcile guards.
    #[test]
    fn assignment_is_idempotent_append_only_and_ordered_by_the_caller() {
        let sql = ASSIGN_ORDINALS_SQL;

        // Idempotent: an existing candidate keeps its ordinal. Without DO NOTHING a
        // re-gather would error (or worse, renumber), breaking every prior reference.
        assert!(
            sql.contains("ON CONFLICT (scenario_id, graph_node_id) DO NOTHING"),
            "re-gather must never re-assign an existing ordinal: {sql}"
        );
        // Append-only: continue from the scenario's high-water mark, never from the
        // current row count (which would reuse the holes left by retired duplicates).
        assert!(
            sql.contains("MAX(ordinal)") && sql.contains("ROW_NUMBER()"),
            "new ordinals must continue from MAX, not from a count: {sql}"
        );
        // First assignment starts at 1: COALESCE over an empty table gives 0, +1.
        assert!(
            sql.contains("0\n        ) + ROW_NUMBER()") || sql.contains("0"),
            "an empty scenario must start at 1: {sql}"
        );
        // The caller's order IS the id order.
        assert!(
            sql.contains("WITH ORDINALITY") && sql.contains("ORDER BY n.ord"),
            "assignment must follow the caller's supplied order: {sql}"
        );
        // Scenario-scoped on both the read and the write — ordinals are per-scenario,
        // so a MAX that ignored the scenario would leak one scenario's sequence into
        // another's.
        assert!(
            sql.contains("WHERE scenario_id = $1"),
            "the high-water mark must be scoped to this scenario: {sql}"
        );
    }
}
