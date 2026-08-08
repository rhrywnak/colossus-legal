//! Theme Scan persistence + summarize (LLM Config Chunk B).
//!
//! Split out of `theme_scan_judge.rs` (module-size limit). Owns two things the
//! judge does not:
//!
//! 1. the `scan_runs` + `scan_run_verdicts` audit writes (every run);
//! 2. the token/cost aggregation and the [`ThemeScanSummary`] the route returns.
//!
//! ## Domain note: SCANNING IS SCORING, NEVER COMMITTING
//!
//! This module deliberately does NOT write `scenario_fact_refs`, and since
//! 2026-08-08 that is the design's foundation rather than a restriction on it.
//! There is exactly ONE write path into a scenario's candidate facts — the
//! HUMAN'S RULING — and a scan reaches the queue by being READ, not by writing:
//! the cards route projects the latest completed run's admitted verdicts as
//! proposals (`services::scenario_card_projection`).
//!
//! Everything that makes that safe depends on the absence of a write here. A junk
//! scan costs nothing to undo, because there is nothing to undo. A re-scan cannot
//! disturb a ruling, because it never touches a row. A run can be deleted and its
//! un-ruled proposals simply stop being projected. The dead-database test in the
//! sibling test module pins the absence, and it is not a formality.
//!
//! (The retired **Merge selected** button was the previous single writer, and the
//! reason a human had to select every candidate twice. It is gone.)
//!
//! That is also why the old `dry_run` distinction is gone rather than defaulted: it
//! existed to answer "should this scan auto-write its picks?", and in this model
//! the answer is permanently no. Nothing here branches on it, so a scan and a
//! former "benchmark" scan are now the same operation.

use sqlx::PgPool;
use uuid::Uuid;

use crate::bias::dto::BiasInstance;
use crate::dto::theme_scan::ScanConservation;
use crate::dto::{ThemeScanRejected, ThemeScanSuggestion, ThemeScanSummary};
use crate::repositories::pipeline_repository::{insert_scan_run_verdicts, ScanRunVerdictRecord};
use crate::services::theme_scan_judge::JudgeOutcome;
use crate::services::theme_scan_parse::Verdict;
use crate::services::theme_scan_prefilter::CandidateGroup;

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
    pub cost_per_input_token: Option<f64>,
    pub cost_per_output_token: Option<f64>,
    pub duration_ms: i64,
    /// Where every gathered row went, measured by the pre-filter before judging
    /// began. Carried through unchanged and frozen into the summary — this run's
    /// record of its own input (task 2.15 Tier 2, item 1c).
    pub conservation: ScanConservation,
}

/// Running tallies + the verdict rows accumulated across one run.
#[derive(Default)]
struct Accumulator {
    /// Verdicts judged relevant. Named for what it counts, not for a side effect:
    /// the scan no longer writes anything to `scenario_fact_refs`, so the former
    /// `relevant_written` described a write that no longer happens.
    relevant: usize,
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
    results: Vec<(CandidateGroup, JudgeOutcome)>,
) -> ThemeScanSummary {
    let candidates_read = results.len();
    let mut acc = Accumulator::default();
    for (group, outcome) in results {
        // Classification is now pure (no DB round-trip per candidate): with the
        // fact-ref write gone, the only I/O left in this module is the single
        // batched `write_verdicts` below.
        process_one(&meta, group, outcome, &mut acc);
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
        input_tokens: acc.input_tokens,
        output_tokens: acc.output_tokens,
        computed_cost,
        duration_ms: meta.duration_ms,
        candidates_read,
        conservation: meta.conservation,
        relevant: acc.relevant,
        irrelevant: acc.irrelevant,
        failed: acc.failed,
        suggestions: acc.suggestions,
        rejected_sample: sample_rejected(acc.rejected, THEME_SCAN_REJECTED_SAMPLE_SIZE),
    }
}

/// Classify one judged GROUP: accumulate its tokens, tally it once, and record a
/// `scan_run_verdicts` row for every node the group speaks for.
///
/// ## Why the verdict FANS OUT to the twins (ruling R2, 2026-08-08)
///
/// The group cost one LLM call because its quotes are byte-identical, but the
/// audit trail is keyed on `graph_node_id` — and the scorecard that measures the
/// scan against Roman's rulings joins his ledger to `scan_run_verdicts` on exactly
/// that key. A twin with no verdict row would be counted as a statement the scan
/// LOST, when in truth the scan judged its text and folded the row deliberately.
/// So each member gets the same verdict, and the run's own tallies count the group
/// ONCE (a scan that judged 124 quotes did not judge 138).
fn process_one(
    meta: &ScanRunMeta,
    group: CandidateGroup,
    outcome: JudgeOutcome,
    acc: &mut Accumulator,
) {
    add_tokens(&mut acc.input_tokens, outcome.input_tokens);
    add_tokens(&mut acc.output_tokens, outcome.output_tokens);

    let CandidateGroup {
        representative,
        members,
    } = group;
    let fields = classify(meta, &representative, &members, &outcome.verdict, acc);

    for graph_node_id in members {
        acc.verdicts.push(ScanRunVerdictRecord {
            run_id: meta.run_id,
            graph_node_id,
            relevant: fields.relevant,
            proposed_role: fields.proposed_role.clone(),
            confidence: fields.confidence,
            reason: fields.reason.clone(),
            // The same reply produced every one of these verdicts; storing it on
            // each row keeps a member's audit record self-contained rather than a
            // pointer to a sibling that a later delete could remove.
            raw_reply: outcome.raw_reply.clone(),
            error: fields.error.clone(),
        });
    }
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
/// (suggested to the human, never written), an irrelevant verdict (sampled, never
/// suggested), or a per-item failure (counted, logged with `evidence_id`).
fn classify(
    meta: &ScanRunMeta,
    candidate: &BiasInstance,
    members: &[String],
    verdict: &Result<Verdict, String>,
    acc: &mut Accumulator,
) -> VerdictFields {
    match verdict {
        Ok(v) if v.relevant => handle_relevant(candidate, members, v, acc),
        Ok(v) => handle_irrelevant(candidate, v, acc),
        Err(reason) => handle_failed(meta, candidate, reason, acc),
    }
}

/// A relevant verdict: tally it and offer it to the human as a suggestion.
///
/// ## What the write removal changed here
///
/// This used to attempt a `scenario_fact_refs` upsert and branch on the outcome:
/// a write failure was counted as a per-item `failed` AND suppressed the
/// suggestion, so a relevant verdict the human had already paid for could vanish
/// from the results list because of a database hiccup. With scanning reduced to
/// scoring, there is no write to fail — every relevant verdict now reaches the
/// human as a checkable suggestion. `failed` is left to mean what its name says:
/// the model could not produce a verdict.
fn handle_relevant(
    candidate: &BiasInstance,
    members: &[String],
    v: &Verdict,
    acc: &mut Accumulator,
) -> VerdictFields {
    acc.relevant += 1;
    acc.suggestions
        .push(to_suggestion(candidate.clone(), members, v));
    VerdictFields {
        relevant: Some(true),
        proposed_role: Some(v.proposed_role.code().to_string()),
        confidence: Some(v.confidence),
        reason: Some(v.reason.clone()),
        // No write to fail, so no per-item error to record. A verdict-level error
        // still lands here via `handle_failed`.
        error: None,
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

/// Map a judged verdict to its wire suggestion (carries the graph card content).
///
/// `covers_node_ids` is the group's whole membership, so a merge of this ONE pick
/// rules every byte-identical twin with it — the human sees one card, and the
/// duplicate does not return tomorrow as an unruled candidate.
fn to_suggestion(
    candidate: BiasInstance,
    members: &[String],
    verdict: &Verdict,
) -> ThemeScanSuggestion {
    ThemeScanSuggestion {
        graph_node_id: candidate.evidence_id.clone(),
        proposed_role: verdict.proposed_role.code().to_string(),
        reason: verdict.reason.clone(),
        confidence: verdict.confidence,
        covers_node_ids: members.to_vec(),
        duplicate_count: members.len(),
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
