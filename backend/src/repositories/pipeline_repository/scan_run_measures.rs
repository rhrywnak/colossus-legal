// =============================================================================
// scan_run_measures.rs — how long a scan actually takes, per model
// =============================================================================
//
// One read, added 2026-09-11 for the scenario-facts header's confirmation bar.
//
// ## Why the estimate is MEASURED and not configured
//
// The confirmation asks a human to authorise 313 metered calls, and "about 55
// minutes" is the half of that sentence which decides whether they start it now
// or after lunch. A configured number would be a guess with a settings row's
// authority: right on the day it was written and quietly wrong after the next
// GPU change, with nothing on the page to say it had drifted.
//
// The run history already knows. Every completed run recorded how long it took
// and how many candidates it judged, so the rate is a division over rows this
// deployment produced itself — it re-measures every time a scan finishes, and a
// model nothing has run yet honestly has no rate at all.
//
// ## Why this is a BACKEND derivation (Standing Rule 12)
//
// The same argument `pool_delta` carries in `scan_run_projection`: this is a
// derivation over the whole history with rules in it (which runs count, how the
// samples combine), and a browser reimplementing them would eventually disagree
// with the server about how long a scan takes. The browser multiplies a number by
// a candidate count; it does not decide what the number means.
//
// ## Why it is a SIBLING module rather than more lines in `scan_runs.rs`
//
// That file sits at 295 non-comment lines against the 300 limit (Rule 17), and
// ruling R6 puts its remediation in its own task. A new read goes beside it.

use std::collections::HashMap;

use sqlx::PgPool;

use super::PipelineRepoError;

/// Seconds per judged candidate, per model, over every completed run.
///
/// ## Why the sums are divided, and not the rates averaged
///
/// `sum(duration_ms) / sum(candidates_judged)` weights each run by how much work
/// it did. The alternative — averaging each run's own rate — gives a nine-card
/// smoke test the same vote as a 313-card scan, so one quick run on a warm cache
/// can halve the estimate the next human is shown.
///
/// ## Why the casts
///
/// `sum()` over a `BIGINT`/`INTEGER` column returns `NUMERIC` in Postgres, and
/// `NUMERIC` is not `f64`-decodable in this tree (no `rust_decimal` or
/// `bigdecimal` feature). `::float8` at the boundary is the house fix — the same
/// one `LIST_SCAN_RUNS_SQL` applies to `computed_cost`.
///
/// `candidates_judged > 0` is a guard against division by zero AND a statement of
/// meaning: a completed run that judged nothing measured nothing, and folding its
/// duration in would report a rate per candidate it never had.
///
/// Query text, not configuration — Rule 13 does not apply. Extracted as a `const`
/// so its shape can be asserted without a live database (the house pattern).
const SECONDS_PER_CANDIDATE_SQL: &str = "SELECT model_id, \
     sum(duration_ms)::float8 / sum(candidates_judged)::float8 / 1000.0 \
     AS seconds_per_candidate \
     FROM scan_runs \
     WHERE status = $1 AND candidates_judged > 0 \
     GROUP BY model_id";

/// What every model that has ever completed a scan measured, keyed by model id.
///
/// A model absent from the map has no completed run with judged candidates —
/// which is a real state, not a zero, and the caller must render it as the
/// ABSENCE of an estimate rather than as a fast one.
///
/// ## Rust Learning: `HashMap` as the return, not `Vec<(String, f64)>`
///
/// The caller's question is "what is the rate for THIS model id?", asked once per
/// entry in the catalogue. A map answers that in one hash lookup; a vector makes
/// every caller write the same linear scan, and the third one writes it wrong.
/// Returning the shape the caller needs is the cheaper contract.
///
/// # Errors
/// Returns [`PipelineRepoError`] when the history cannot be read. Deliberately NOT
/// degraded to an empty map: an unreadable history and a history with nothing in
/// it would then be one state, and the page would silently drop the time clause
/// from every confirmation without anyone learning why (Standing Rule 1).
pub async fn seconds_per_candidate_by_model(
    pool: &PgPool,
) -> Result<HashMap<String, f64>, PipelineRepoError> {
    let rows: Vec<(String, f64)> = sqlx::query_as(SECONDS_PER_CANDIDATE_SQL)
        .bind(super::scan_runs::SCAN_STATUS_COMPLETED)
        .fetch_all(pool)
        .await?;

    Ok(rows.into_iter().collect())
}

#[cfg(test)]
#[path = "scan_run_measures_tests.rs"]
mod tests;
