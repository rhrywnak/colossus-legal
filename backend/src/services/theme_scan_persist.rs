//! Theme Scan persistence + summarize (LLM Config Chunk B).
//!
//! Split out of `theme_scan_judge.rs` (module-size limit). Owns three things the
//! judge does not:
//!
//! 1. the `scenario_fact_refs` upsert for RELEVANT verdicts — SUPPRESSED on a
//!    `dry_run` (A4: so a benchmark's two model runs do not collide on the
//!    `(scenario_id, graph_node_id)` PK);
//! 2. the `scan_runs` + `scan_run_verdicts` audit writes (EVERY run, dry or not);
//! 3. the token/cost aggregation and the [`ThemeScanSummary`] the route returns.

use sqlx::PgPool;
use uuid::Uuid;

use crate::bias::dto::BiasInstance;
use crate::dto::{ThemeScanRejected, ThemeScanSuggestion, ThemeScanSummary};
use crate::repositories::pipeline_repository::{
    insert_scan_run_verdicts, reconcile_fact_ref, PipelineRepoError, ScanRunVerdictRecord,
};
use crate::services::theme_scan_judge::JudgeOutcome;
use crate::services::theme_scan_parse::Verdict;

// CONST: honesty-check sample size — a fixed UX constant, not a deployment knob.
// Bounds how many rejected quotes ride inline in the response for a human
// spot-check; ten is a reviewable handful (moved here with the persist logic).
const THEME_SCAN_REJECTED_SAMPLE_SIZE: usize = 10;

/// The per-run facts the persist pass needs. The `scan_runs` header row already
/// exists as `running` (inserted at start with `resolved_params`/`started_at`),
/// so those are NOT here — persist writes verdicts + `scenario_fact_refs` and
/// builds the summary; the caller finalizes the header. The per-token costs feed
/// [`compute_cost`]; `duration_ms` (the judging elapsed) lands in the summary.
pub(crate) struct ScanRunMeta {
    pub run_id: Uuid,
    pub scenario_id: Uuid,
    pub model_id: String,
    pub dry_run: bool,
    pub cost_per_input_token: Option<f64>,
    pub cost_per_output_token: Option<f64>,
    pub duration_ms: i64,
}

/// Running tallies + the verdict rows accumulated across one run.
#[derive(Default)]
struct Accumulator {
    relevant_written: usize,
    irrelevant: usize,
    failed: usize,
    suggestions: Vec<ThemeScanSuggestion>,
    rejected: Vec<ThemeScanRejected>,
    verdicts: Vec<ScanRunVerdictRecord>,
    // NULL-if-absent token sums (never a fabricated 0 — Standing Rule 1).
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
}

/// Persist every verdict, write the audit tables, and build the summary.
pub(crate) async fn persist_and_summarize(
    pool: &PgPool,
    meta: ScanRunMeta,
    results: Vec<(BiasInstance, JudgeOutcome)>,
) -> ThemeScanSummary {
    let candidates_read = results.len();
    let mut acc = Accumulator::default();
    for (candidate, outcome) in results {
        process_one(pool, &meta, candidate, outcome, &mut acc).await;
    }

    let computed_cost = compute_cost(
        acc.input_tokens,
        acc.output_tokens,
        meta.cost_per_input_token,
        meta.cost_per_output_token,
    );

    write_verdicts(pool, &meta, &acc.verdicts).await;

    ThemeScanSummary {
        run_id: meta.run_id,
        model_id: meta.model_id,
        dry_run: meta.dry_run,
        input_tokens: acc.input_tokens,
        output_tokens: acc.output_tokens,
        computed_cost,
        duration_ms: meta.duration_ms,
        candidates_read,
        relevant_written: acc.relevant_written,
        irrelevant: acc.irrelevant,
        failed: acc.failed,
        suggestions: acc.suggestions,
        rejected_sample: sample_rejected(acc.rejected, THEME_SCAN_REJECTED_SAMPLE_SIZE),
    }
}

/// Classify one judged candidate: accumulate its tokens, apply the
/// `scenario_fact_refs` write (unless dry-run), tally it, and record its
/// `scan_run_verdicts` row.
async fn process_one(
    pool: &PgPool,
    meta: &ScanRunMeta,
    candidate: BiasInstance,
    outcome: JudgeOutcome,
    acc: &mut Accumulator,
) {
    add_tokens(&mut acc.input_tokens, outcome.input_tokens);
    add_tokens(&mut acc.output_tokens, outcome.output_tokens);

    let fields = classify(pool, meta, &candidate, &outcome.verdict, acc).await;

    acc.verdicts.push(ScanRunVerdictRecord {
        run_id: meta.run_id,
        graph_node_id: candidate.evidence_id,
        relevant: fields.relevant,
        proposed_role: fields.proposed_role,
        confidence: fields.confidence,
        reason: fields.reason,
        raw_reply: outcome.raw_reply,
        error: fields.error,
    });
}

/// The verdict-row fields for one candidate (mirrors `scan_run_verdicts`).
struct VerdictFields {
    relevant: Option<bool>,
    proposed_role: Option<String>,
    confidence: Option<f32>,
    reason: Option<String>,
    error: Option<String>,
}

/// Route one candidate into the tally and produce its verdict-row fields.
///
/// Three outcomes (Standing Rule 1 — distinguishable): a relevant verdict
/// (written unless dry-run), an irrelevant verdict (sampled, never written), or a
/// per-item failure (counted, logged with `evidence_id`).
async fn classify(
    pool: &PgPool,
    meta: &ScanRunMeta,
    candidate: &BiasInstance,
    verdict: &Result<Verdict, String>,
    acc: &mut Accumulator,
) -> VerdictFields {
    match verdict {
        Ok(v) if v.relevant => handle_relevant(pool, meta, candidate, v, acc).await,
        Ok(v) => handle_irrelevant(candidate, v, acc),
        Err(reason) => handle_failed(meta, candidate, reason, acc),
    }
}

/// A relevant verdict: write it (unless dry-run) and tally. A non-dry write
/// failure is a counted per-item failure that still records the verdict values.
async fn handle_relevant(
    pool: &PgPool,
    meta: &ScanRunMeta,
    candidate: &BiasInstance,
    v: &Verdict,
    acc: &mut Accumulator,
) -> VerdictFields {
    let write_err = maybe_write_relevant(pool, meta, candidate, v).await;
    match write_err {
        None => {
            acc.relevant_written += 1;
            acc.suggestions.push(to_suggestion(candidate.clone(), v));
        }
        Some(_) => acc.failed += 1,
    }
    VerdictFields {
        relevant: Some(true),
        proposed_role: Some(v.proposed_role.code().to_string()),
        confidence: Some(v.confidence),
        reason: Some(v.reason.clone()),
        error: write_err,
    }
}

/// An irrelevant verdict: never written, but sampled for the honesty check.
fn handle_irrelevant(
    candidate: &BiasInstance,
    v: &Verdict,
    acc: &mut Accumulator,
) -> VerdictFields {
    acc.irrelevant += 1;
    acc.rejected.push(ThemeScanRejected {
        graph_node_id: candidate.evidence_id.clone(),
        reason: v.reason.clone(),
        confidence: v.confidence,
        content: candidate.clone(),
    });
    VerdictFields {
        relevant: Some(false),
        proposed_role: Some(v.proposed_role.code().to_string()),
        confidence: Some(v.confidence),
        reason: Some(v.reason.clone()),
        error: None,
    }
}

/// A per-item failure: counted and logged with run/evidence/scenario context.
fn handle_failed(
    meta: &ScanRunMeta,
    candidate: &BiasInstance,
    reason: &str,
    acc: &mut Accumulator,
) -> VerdictFields {
    acc.failed += 1;
    tracing::error!(
        run_id = %meta.run_id,
        evidence_id = %candidate.evidence_id,
        scenario_id = %meta.scenario_id,
        reason = %reason,
        "theme scan: producing a verdict failed"
    );
    VerdictFields {
        relevant: None,
        proposed_role: None,
        confidence: None,
        reason: None,
        error: Some(reason.to_string()),
    }
}

/// Upsert a relevant verdict as an `undecided` suggestion — unless this is a
/// dry (benchmark) run, in which case NOTHING is written to `scenario_fact_refs`
/// (A4). Returns `Some(error)` only on a real write failure (logged here).
async fn maybe_write_relevant(
    pool: &PgPool,
    meta: &ScanRunMeta,
    candidate: &BiasInstance,
    verdict: &Verdict,
) -> Option<String> {
    if meta.dry_run {
        return None;
    }
    match write_relevant(pool, meta.scenario_id, candidate, verdict).await {
        Ok(()) => None,
        Err(e) => {
            let msg = format!("scenario_fact_refs write failed: {e}");
            tracing::error!(
                run_id = %meta.run_id,
                evidence_id = %candidate.evidence_id,
                scenario_id = %meta.scenario_id,
                error = %e,
                "theme scan: writing a relevant verdict failed"
            );
            Some(msg)
        }
    }
}

/// Reconcile one relevant verdict as an `undecided` suggestion (awaits a human
/// include/drop ruling), status-preserving.
///
/// ## Why `reconcile_fact_ref`, not `upsert_fact_ref` (latent-bug fix)
///
/// `upsert_fact_ref` overwrites `status = EXCLUDED.status` on conflict, so a
/// non-dry RE-scan used to silently reset a candidate the human had already
/// `Included` or `Dropped` back to `undecided` — destroying curation on every
/// re-run. `reconcile_fact_ref` refreshes only the LLM layer (role + confidence)
/// and leaves an `included`/`dropped` row untouched, so re-scans are now safe.
///
/// ## Why no `note`
///
/// The reconcile deliberately never writes `note` — that column is the human's.
/// The model's `reason` is NOT lost: it is stored per-candidate in
/// `scan_run_verdicts.reason` (the audit record) and rides the live
/// `ThemeScanSuggestion` card. This drops the previous (mis)use of `note` to
/// carry the LLM reason onto the fact ref.
async fn write_relevant(
    pool: &PgPool,
    scenario_id: Uuid,
    candidate: &BiasInstance,
    verdict: &Verdict,
) -> Result<(), PipelineRepoError> {
    reconcile_fact_ref(
        pool,
        scenario_id,
        &candidate.evidence_id,
        Some(verdict.proposed_role.code()),
        Some(verdict.confidence),
    )
    .await
}

/// Write the `scan_run_verdicts` detail rows (the `scan_runs` header already
/// exists as `running`; the caller finalizes it separately).
///
/// Best-effort but LOUD: a DB failure here is logged with the run id and does NOT
/// discard the summary the client will earn (the scan spent real budget). A
/// missing verdict set is an operator-visible error, not a silent gap.
async fn write_verdicts(pool: &PgPool, meta: &ScanRunMeta, verdicts: &[ScanRunVerdictRecord]) {
    if let Err(e) = insert_scan_run_verdicts(pool, verdicts).await {
        tracing::error!(run_id = %meta.run_id, scenario_id = %meta.scenario_id, error = %e,
            "theme scan: writing scan_run_verdicts failed (results still returned)");
    }
}

/// Map a written verdict to its wire suggestion (carries the graph card content).
fn to_suggestion(candidate: BiasInstance, verdict: &Verdict) -> ThemeScanSuggestion {
    ThemeScanSuggestion {
        graph_node_id: candidate.evidence_id.clone(),
        proposed_role: verdict.proposed_role.code().to_string(),
        reason: verdict.reason.clone(),
        confidence: verdict.confidence,
        content: candidate,
    }
}

/// Add a candidate's reported token count into a running NULL-if-absent sum.
///
/// `None` stays `None` until the first reported value, so a run where no call
/// reported usage yields `NULL` (distinct from a real 0). `u32 -> i64` widens
/// via `i64::from` (infallible), never an `as`-cast.
fn add_tokens(sum: &mut Option<i64>, reported: Option<u32>) {
    if let Some(t) = reported {
        *sum = Some(sum.unwrap_or(0) + i64::from(t));
    }
}

/// Compute dollar cost = input×cost_in + output×cost_out, when everything is
/// known. `None` if either per-token cost is absent (local vLLM) or either token
/// sum is absent — an honest "unknown", never a fabricated 0.
fn compute_cost(
    input: Option<i64>,
    output: Option<i64>,
    cost_in: Option<f64>,
    cost_out: Option<f64>,
) -> Option<f64> {
    let (ci, co) = (cost_in?, cost_out?);
    Some(tokens_to_f64(input?) * ci + tokens_to_f64(output?) * co)
}

/// `i64` token count → `f64` without an `as`-cast. Token counts fit `i32` (a run
/// never approaches 2.1B tokens), whose `f64` conversion is exact and infallible;
/// the impossible overflow degrades to `0.0`, keeping cost finite rather than
/// panicking.
fn tokens_to_f64(tokens: i64) -> f64 {
    i32::try_from(tokens).map(f64::from).unwrap_or(0.0)
}

/// Narrow a `usize` count to the `INTEGER` column type. A scan never approaches
/// `i32::MAX` candidates; the impossible overflow is logged and capped rather
/// than silently wrapping (Standing Rule 1). `pub(crate)` so the finalize step
/// (in `theme_scan`) reuses the same conversion for the header counts.
pub(crate) fn count_to_i32(n: usize, field: &str) -> i32 {
    i32::try_from(n).unwrap_or_else(|_| {
        tracing::error!(field, value = n, "theme scan: count exceeded i32 — capped");
        i32::MAX
    })
}

/// Take an evenly-spread sample of at most `max` rejected quotes.
///
/// A strided pick (indices `k * n / max`) spreads the sample across the whole
/// reject set (ordered by `evidence_id`); the first-`max` alternative would bias
/// the honesty check toward one end of the id space. No RNG dependency — the
/// check wants a representative spread, not cryptographic randomness.
fn sample_rejected(rejected: Vec<ThemeScanRejected>, max: usize) -> Vec<ThemeScanRejected> {
    let n = rejected.len();
    if n <= max {
        return rejected;
    }
    (0..max).map(|k| rejected[k * n / max].clone()).collect()
}

#[cfg(test)]
#[path = "theme_scan_persist_tests.rs"]
mod tests;
