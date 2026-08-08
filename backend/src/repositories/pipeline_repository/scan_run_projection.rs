//! The two reads the PROPOSAL PROJECTION is built from (task: scan → ruling).
//!
//! ## Why a new module rather than more functions on `scan_runs.rs`
//!
//! Two reasons, and the second is the real one. `scenario_store.rs` — the other
//! plausible home — is already over the 300-line limit, so adding to it would
//! deepen an existing violation. And these two reads are a distinct SUBJECT: they
//! answer "what does a completed run currently propose to this scenario's queue?",
//! which is a read the queue makes on every page load and which nothing else in
//! the run lifecycle cares about. `scan_runs.rs` owns the run's own life (birth,
//! progress, finalize, delete); this owns the run's shadow on the curation surface.
//!
//! ## The law these two reads encode (R-b)
//!
//! **Only the latest COMPLETED run projects.** No unions across runs, and a run
//! that failed or is still running projects nothing. That rule lives in the SQL
//! here — one place — rather than in whichever caller happens to need it, because
//! a second copy of "which run counts" is a second answer waiting to disagree.
//!
//! CRITICAL: both tables live in the pipeline database (`colossus_legal_v2`), so
//! callers pass `&state.pipeline_pool`.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

/// The completed run whose verdicts a scenario's queue is currently showing.
///
/// Carries only what the SURFACE needs to attribute the proposals: which run, what
/// judged them, and when it started. The counts stay in the run header — this is
/// not a second copy of the history row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProjectingRunRow {
    pub run_id: Uuid,
    pub model_id: String,
    pub started_at: DateTime<Utc>,
}

// CONST: the projecting-run query, held as a `const` for the house SQL-shape test
// pattern (see `LIST_SCAN_RUNS_SQL`). Query text, not config — Rule 13 N/A.
//
// `status = $2` is R-b's whole enforcement: a `running` or `failed` run is not a
// candidate here, so a scan that died at the vLLM gate proposes nothing.
//
// ORDER BY started_at DESC matches the history table the human is reading, so
// "the latest run" means the same thing on both surfaces. There is no completion
// timestamp on `scan_runs` to order by instead, and scans are serialised per
// scenario (one active run, swept at boot), so start order IS completion order in
// practice. Stated here rather than silently assumed.
const PROJECTING_RUN_SQL: &str = "SELECT run_id, model_id, started_at \
     FROM scan_runs WHERE scenario_id = $1 AND status = $2 \
     ORDER BY started_at DESC LIMIT 1";

/// The scenario's latest COMPLETED run, or `None` when nothing has completed.
///
/// `None` is a real answer, not a gap: a scenario that has never been scanned —
/// or whose only run failed — proposes nothing, and the queue must render that as
/// "no proposals" rather than as an error. A failure to READ is a different thing
/// entirely and propagates (see the caller).
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn fetch_projecting_run(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Option<ProjectingRunRow>, PipelineRepoError> {
    let row = sqlx::query_as::<_, ProjectingRunRow>(PROJECTING_RUN_SQL)
        .bind(scenario_id)
        .bind(super::scan_runs::SCAN_STATUS_COMPLETED)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

/// One admitted verdict, reduced to what a proposed card needs.
///
/// ## Why the three payload columns stay `Option`
///
/// They are `Option` in the table, and the reason is Standing Rule 1: a row whose
/// judging FAILED carries `relevant = NULL` and an `error`, and a row that
/// succeeded carries all three. This query only reads `relevant = true` rows, so
/// in practice all three are present — but the type does not pretend to a
/// guarantee the schema does not make, and the assembler decides what a proposal
/// with no reason looks like rather than a decode inventing one.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RelevantVerdictRow {
    pub graph_node_id: String,
    pub proposed_role: Option<String>,
    /// Postgres `REAL` → `f32`. Never reaches the wire as a number: the card
    /// bands it (§7.8, "never a naked percentage").
    pub confidence: Option<f32>,
    pub reason: Option<String>,
}

// CONST: the admitted-verdict read. `relevant = true` is the "admitted" filter —
// an irrelevant verdict was judged and rejected, and re-proposing it would undo
// the judgement the human paid for. Query text, not config — Rule 13 N/A.
const RELEVANT_VERDICTS_SQL: &str = "SELECT graph_node_id, proposed_role, confidence, reason \
     FROM scan_run_verdicts WHERE run_id = $1 AND relevant = true";

/// Every admitted verdict of one run — the raw material of the projection.
///
/// Returned unfolded, one row per graph node INCLUDING byte-identical twins (the
/// scan writes a verdict row per member deliberately; see
/// `services::theme_scan_persist`). Folding is the pure assembler's job, not the
/// query's, because the fold key is the quote text and that lives in the graph.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn list_relevant_verdicts_for_run(
    pool: &PgPool,
    run_id: Uuid,
) -> Result<Vec<RelevantVerdictRow>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, RelevantVerdictRow>(RELEVANT_VERDICTS_SQL)
        .bind(run_id)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    // These two laws are enforced by the DATABASE, so they are asserted against the
    // statement rather than against a returned value — the house pattern for a
    // rule that lives in SQL (see `merge_sql_is_relevant_only_and_fenced_to_run_and_scenario`
    // and `documents_delete`). A unit test cannot run a query; it can prove the
    // query cannot express the wrong thing.

    #[test]
    fn only_the_latest_completed_run_projects() {
        // R-b: no unions. One scenario, one projecting run, and it is the newest —
        // a re-scan supersedes the previous run's un-ruled proposals rather than
        // adding to them.
        assert!(
            PROJECTING_RUN_SQL.contains("ORDER BY started_at DESC")
                && PROJECTING_RUN_SQL.contains("LIMIT 1"),
            "exactly ONE run projects, and it is the newest: {PROJECTING_RUN_SQL}"
        );
        assert!(
            PROJECTING_RUN_SQL.contains("scenario_id = $1"),
            "the projecting run must be fenced to its scenario: {PROJECTING_RUN_SQL}"
        );
    }

    #[test]
    fn a_failed_run_projects_nothing() {
        // A run that died at the vLLM gate judged nothing, and a running one is
        // mid-flight. Either putting proposals in front of a human would claim a
        // scan parentage no completed verdict supports — the same reasoning
        // `count_completed_scan_runs` states for the never-scanned notice.
        assert!(
            PROJECTING_RUN_SQL.contains("status = $2"),
            "the projecting-run query must gate on status: {PROJECTING_RUN_SQL}"
        );
        assert_eq!(
            super::super::scan_runs::SCAN_STATUS_COMPLETED,
            "completed",
            "the bound status is the one `finalize_scan_run_completed` writes"
        );
    }

    #[test]
    fn only_admitted_verdicts_are_projected() {
        // An irrelevant verdict was judged and rejected. Projecting it would put a
        // candidate the model already dismissed back in front of the human, which
        // is the opposite of what the scan was paid for.
        assert!(
            RELEVANT_VERDICTS_SQL.contains("relevant = true"),
            "the verdict read must admit only relevant rows: {RELEVANT_VERDICTS_SQL}"
        );
        assert!(
            RELEVANT_VERDICTS_SQL.contains("run_id = $1"),
            "the verdict read must be fenced to ONE run (R-b): {RELEVANT_VERDICTS_SQL}"
        );
    }
}
