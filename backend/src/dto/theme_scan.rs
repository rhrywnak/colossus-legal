//! Theme Scan response DTOs (D2b).
//!
//! The wire shape returned by
//! `POST /cases/:slug/scenarios/:scenario_id/theme-scan`. A scan reads every
//! candidate quote about the scenario's subject, asks the LLM judge to rate each
//! one against the scenario's `attack_meaning`, writes the RELEVANT verdicts to
//! `scenario_fact_refs` (as `confirmed=false` suggestions), and returns this
//! summary so D3 can render the suggestions and the honesty-check sample.
//!
//! ## Why the rejected sample rides in the response, not a second endpoint
//!
//! Amendment 1 wants an honesty check: after a scan, show a sample of the quotes
//! the judge REJECTED, so a human can confirm the judge is not silently dropping
//! relevant evidence. We do NOT persist irrelevant verdicts (only relevant ones
//! become `scenario_fact_refs` rows), so the rejected quotes exist only in the
//! scan's own memory. Returning a bounded sample INLINE here is therefore the
//! cheapest honest design — the scan already judged them; a separate "sample
//! rejects" endpoint would have to re-run the whole scan to reconstruct the set.
//!
//! ## Why `content` is a `BiasInstance`
//!
//! A `scenario_fact_refs` row stores only a `graph_node_id`; the human-readable
//! content (quote, speaker, document, pattern tags) lives in the graph. The Bias
//! Explorer already assembles exactly that into `BiasInstance`, and the fact
//! curation DTOs (`ScenarioFactDto`) already reuse it — so one frontend card
//! renders a bias candidate, a saved fact, AND a scan suggestion, with one
//! graph→content mapping rather than three that can drift.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::bias::dto::BiasInstance;

/// Optional request body for `POST .../theme-scan`.
///
/// Both fields are optional so an EMPTY body preserves the pre-Chunk-B behavior
/// (default model, non-dry-run). The handler accepts `Option<Json<ScanRequest>>`
/// and falls back to `ScanRequest::default()` when the body is absent.
///
/// ## Rust Learning: `#[serde(default)]` = "this field is optional on the wire"
///
/// With `#[serde(default)]`, a missing key deserializes to the field type's
/// `Default` (`None` for `model_id`, `false` for `dry_run`) instead of failing.
/// The DERIVE of `Default` on the struct then lets the handler synthesize the
/// whole request when there is no body at all. Absence is legitimate here — the
/// meaningful distinction (Standing Rule 1) is model-picked-or-default, captured
/// by `Option`, and dry-run-or-not, captured by the bool.
// serde: deny_unknown_fields — an unknown key in a client request body is a
// caller mistake (a typo'd `model` for `model_id`, a stale field), and silently
// ignoring it would let a misspelled `dry_run` run a live scan. Reject with 400.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanRequest {
    /// The `llm_models.id` to judge with. `None` → the per-feature default
    /// (`THEME_SCAN_MODEL`, else the chat default).
    #[serde(default)]
    pub model_id: Option<String>,
    /// `true` = benchmark run: judge + record to the audit tables, but do NOT
    /// upsert `scenario_fact_refs`. `false` (default) = normal workbench scan.
    #[serde(default)]
    pub dry_run: bool,
}

/// The immediate response to `POST .../theme-scan` — the scan now runs in the
/// BACKGROUND, so the POST returns a handle instead of blocking for the summary.
///
/// The client polls `GET .../scan-runs/:run_id` (returning [`ScanRunStatusResponse`])
/// until `status == "completed"`. `candidates_total` is the progress denominator,
/// known up front, so the UI can render "judged 0 of N" immediately.
#[derive(Debug, Clone, Serialize)]
pub struct ScanStartedResponse {
    pub run_id: Uuid,
    /// Always `"running"` here — the job was just spawned.
    pub status: String,
    pub candidates_total: i32,
}

/// The poll response for `GET .../scan-runs/:run_id`.
///
/// While `running`, the counts are a LIVE, advancing ESTIMATE (the UI must show
/// them as in-progress, not final) and `summary` is `None`. Once `completed`, the
/// counts are authoritative and `summary` carries the full [`ThemeScanSummary`].
/// On `failed`, `error` says why (Standing Rule 1).
#[derive(Debug, Clone, Serialize)]
pub struct ScanRunStatusResponse {
    pub run_id: Uuid,
    /// `running` | `completed` | `failed`.
    pub status: String,
    pub model_id: String,
    pub dry_run: bool,
    /// Progress denominator (may be absent only for pre-background legacy rows).
    pub candidates_total: Option<i32>,
    /// Progress numerator — how many candidates have been judged so far.
    pub candidates_judged: i32,
    /// Live-then-final outcome counts (see the struct doc).
    pub relevant_count: i32,
    pub irrelevant_count: i32,
    pub failed_count: i32,
    /// The failure reason when `status == "failed"`; `None` otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The full [`ThemeScanSummary`] once `completed`, passed through VERBATIM as
    /// the stored `summary_json` (a render convenience — the GET never re-queries
    /// Neo4j). `None` while running/failed. The wire shape equals `ThemeScanSummary`;
    /// the frontend types it as such.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<serde_json::Value>,
}

/// One row of the scan-run HISTORY list for
/// `GET .../scan-runs` — a lightweight HEADER, deliberately NOT the full result.
///
/// ## Why headers only (not `summary_json`, not verdicts)
///
/// The history list renders a compact per-run row (model, benchmark/real,
/// counts, timestamp) for every run of a scenario — potentially many. Shipping
/// each run's full `ThemeScanSummary` (suggestions + rejected sample, ~dozens of
/// `BiasInstance`s) in the list would make it heavy for no benefit: the detail is
/// fetched lazily via the EXISTING `GET .../scan-runs/:run_id` when a row is
/// clicked. So this DTO carries only what a row shows. It is the retrieval
/// counterpart to [`ScanRunStatusResponse`]; the two overlap on the header
/// fields but differ in intent (one polls a single run, one lists them all).
///
/// `computed_cost` is `Option<f64>` (a null cost is meaningful — a local vLLM
/// model has no per-token cost, or token usage was absent; Standing Rule 1) and
/// is emitted as `null` rather than skipped so the frontend distinguishes
/// "no cost" from a missing field. `started_at` drives the newest-first order.
#[derive(Debug, Clone, Serialize)]
pub struct ScanRunHeader {
    pub run_id: Uuid,
    pub model_id: String,
    pub dry_run: bool,
    /// `running` | `completed` | `failed`.
    pub status: String,
    /// Progress denominator (absent only for pre-background legacy rows).
    pub candidates_total: Option<i32>,
    pub candidates_judged: i32,
    pub relevant_count: i32,
    pub irrelevant_count: i32,
    pub failed_count: i32,
    /// Computed dollar cost, or `null` (local model / no token usage).
    pub computed_cost: Option<f64>,
    pub duration_ms: i64,
    pub started_at: DateTime<Utc>,
    /// How many times this run has been merged into its scenario (`0` = never).
    /// Drives the run detail: `0` → "Merge into scenario"; `>0` → "Merged N× ·
    /// last …" plus an explicit Re-merge. Additive field (backward compatible).
    pub merge_count: i64,
    /// The most recent merge time, or `null` when never merged. Emitted as `null`
    /// (not skipped) so the frontend distinguishes "never merged" from a missing
    /// field — Standing Rule 1.
    pub last_merged_at: Option<DateTime<Utc>>,
}

/// Response for `GET .../scan-runs` — the scenario's run history, newest first.
///
/// Wrapped in `{ runs: [...] }` (rather than a bare array) to mirror the
/// `/api/scan/models` `{ models: [...] }` shape and to leave room for list-level
/// metadata later without a breaking change.
#[derive(Debug, Clone, Serialize)]
pub struct ScanRunListResponse {
    pub runs: Vec<ScanRunHeader>,
}

/// Result of merging one stored run's relevant picks into a scenario.
///
/// `merged` is the number of candidate facts inserted or refreshed as `undecided`
/// suggestions — picks preserved as existing human `included`/`dropped` curation
/// are deliberately NOT counted (see `merge_scan_run_into_scenario`). `i64`
/// because the source `rows_affected` is a `u64` count; a scan's ~94-candidate
/// ceiling is nowhere near the `i64` range, so the cast is lossless.
#[derive(Debug, Clone, Serialize)]
pub struct ScanRunMergeResponse {
    pub merged: i64,
}

/// Result of one Theme Scan run.
///
/// The four counts are exhaustive and non-overlapping:
/// `candidates_read == relevant_written + irrelevant + failed`. This identity is
/// the recall guarantee made observable (Standing Rule 1) — every candidate the
/// scan read lands in exactly one bucket, so a dropped quote would show up as a
/// count that does not add up rather than as a silent absence.
#[derive(Debug, Clone, Serialize)]
pub struct ThemeScanSummary {
    /// The `scan_runs.run_id` this scan recorded — the handle the benchmark
    /// comparison query joins on. Present on every run (dry or not).
    pub run_id: Uuid,
    /// The `llm_models.id` this run judged with (the resolved model, after the
    /// request/`THEME_SCAN_MODEL`/chat-default fallback).
    pub model_id: String,
    /// Whether this was a dry (benchmark) run. When `true`, `relevant_written`
    /// counts relevant verdicts that were RECORDED but NOT upserted into
    /// `scenario_fact_refs` (A4).
    pub dry_run: bool,
    /// Summed reported input tokens across the run; `None` if no call reported
    /// usage (never a fabricated 0).
    pub input_tokens: Option<i64>,
    /// Summed reported output tokens; `None` if no call reported usage.
    pub output_tokens: Option<i64>,
    /// Computed dollar cost (tokens × per-token cost) when known; `None` for a
    /// local vLLM model or absent usage.
    pub computed_cost: Option<f64>,
    /// Wall-clock duration of the judging fan-out in milliseconds (computed at
    /// completion). Lets the benchmark compare Opus vs Qwen latency.
    pub duration_ms: i64,
    /// Total candidate quotes read for the subject (the ungated
    /// `all_evidence_about_subject` count — every Evidence ABOUT the subject).
    pub candidates_read: usize,
    /// Verdicts judged RELEVANT and successfully written to
    /// `scenario_fact_refs` as `confirmed=false` suggestions.
    pub relevant_written: usize,
    /// Verdicts judged NOT relevant to the accusation. Not written; a sample is
    /// surfaced in [`Self::rejected_sample`] for the honesty check.
    pub irrelevant: usize,
    /// Candidates whose verdict could not be produced: an LLM call that
    /// exhausted retries, a reply that failed the strict parse, an out-of-set
    /// role, or a write that failed. Counted, never silently dropped — each is
    /// logged with its `evidence_id` and cause (Standing Rule 1).
    pub failed: usize,
    /// The written suggestions, so the client can render them without a second
    /// round-trip. One entry per `relevant_written`.
    pub suggestions: Vec<ThemeScanSuggestion>,
    /// A bounded, spread-out sample of the rejected quotes for the Amendment-1
    /// honesty check. Empty when nothing was rejected; at most
    /// `THEME_SCAN_REJECTED_SAMPLE_SIZE` entries otherwise.
    pub rejected_sample: Vec<ThemeScanRejected>,
}

/// One RELEVANT verdict written to `scenario_fact_refs`.
#[derive(Debug, Clone, Serialize)]
pub struct ThemeScanSuggestion {
    /// The Evidence node id this suggestion references (the `graph_node_id`
    /// column, equal to the candidate's `evidence_id`).
    pub graph_node_id: String,
    /// The role the judge assigned — one of the four `FactRole` tokens
    /// (`supports` / `corroborates` / `contradicts` / `rebuts`).
    pub proposed_role: String,
    /// The judge's one-to-two-sentence justification (stored in the row's
    /// `note` column).
    pub reason: String,
    /// The judge's self-reported confidence in `[0.0, 1.0]` (stored in the
    /// row's `confidence` column).
    pub confidence: f32,
    /// Live graph card content for the referenced node, so the client renders
    /// the suggestion as a normal fact card.
    pub content: BiasInstance,
}

/// One REJECTED quote, surfaced for the honesty check (never persisted).
#[derive(Debug, Clone, Serialize)]
pub struct ThemeScanRejected {
    /// The Evidence node id the judge rejected.
    pub graph_node_id: String,
    /// Why the judge deemed it not relevant — lets a human sanity-check that the
    /// rejection was sound rather than a missed connection.
    pub reason: String,
    /// The judge's confidence in the (irrelevant) verdict, `[0.0, 1.0]`.
    pub confidence: f32,
    /// Live graph card content for the rejected node.
    pub content: BiasInstance,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_status_omits_error_and_summary_keys() {
        // The frontend's conditional rendering relies on `error`/`summary` being
        // ABSENT (not null) while running — pin the skip_serializing_if contract.
        let running = ScanRunStatusResponse {
            run_id: Uuid::nil(),
            status: "running".to_string(),
            model_id: "m".to_string(),
            dry_run: true,
            candidates_total: Some(94),
            candidates_judged: 10,
            relevant_count: 3,
            irrelevant_count: 6,
            failed_count: 1,
            error: None,
            summary: None,
        };
        let v = serde_json::to_value(&running).expect("serializes");
        assert!(
            v.get("error").is_none(),
            "error key must be omitted when None"
        );
        assert!(
            v.get("summary").is_none(),
            "summary key must be omitted when None"
        );
        assert_eq!(v["candidates_judged"], 10);
    }
}
