//! Theme Scan response DTOs (D2b).
//!
//! The wire shape returned by
//! `POST /cases/:slug/scenarios/:scenario_id/theme-scan`. A scan reads every
//! candidate quote about the scenario's subject, asks the LLM judge to rate each
//! one against the scenario's `attack_meaning`, records every verdict to
//! `scan_run_verdicts`, and returns this summary so the panel can render the
//! relevant picks and the honesty-check sample.
//!
//! ## Scanning is scoring, never committing
//!
//! A scan writes NOTHING to `scenario_fact_refs`. The relevant picks are
//! SUGGESTIONS awaiting the human's explicit **Merge selected**, which is the only
//! path from a verdict into the scenario's candidate facts. Read every count in
//! this module as "what the model thought", never as "what was added to the case".
//!
//! ## Why the rejected sample rides in the response, not a second endpoint
//!
//! Amendment 1 wants an honesty check: after a scan, show a sample of the quotes
//! the judge REJECTED, so a human can confirm the judge is not silently dropping
//! relevant evidence. The rejected quotes' CONTENT lives only in the graph and in
//! this response (the verdict itself is recorded in `scan_run_verdicts`), so
//! returning a bounded sample INLINE is the cheapest honest design — the scan
//! already judged them; a separate "sample rejects" endpoint would have to re-read
//! and re-assemble the whole set.
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
/// The one field is optional so an EMPTY body means "scan with the default
/// model". The handler accepts `Option<Json<ScanRequest>>` and falls back to
/// `ScanRequest::default()` when the body is absent.
///
/// ## Why there is no `dry_run` any more
///
/// `dry_run` used to ask "should this scan auto-write its picks?". Under the
/// unified merge model no scan ever writes — merge is the only write path — so
/// the question has one permanent answer and the field described a distinction
/// that no longer exists. A stale client still sending `dry_run` gets a 400 from
/// `deny_unknown_fields` rather than a silently-ignored key: loud beats silent
/// (Standing Rule 1), and the caller learns their assumption is out of date
/// instead of believing they suppressed a write that was never going to happen.
///
/// ## Rust Learning: `#[serde(default)]` = "this field is optional on the wire"
///
/// With `#[serde(default)]`, a missing key deserializes to the field type's
/// `Default` (`None` for `model_id`) instead of failing. The DERIVE of `Default`
/// on the struct then lets the handler synthesize the whole request when there is
/// no body at all. Absence is legitimate here — the meaningful distinction
/// (Standing Rule 1) is model-picked-or-default, captured by the `Option`.
// serde: deny_unknown_fields — an unknown key in a client request body is a
// caller mistake (a typo'd `model` for `model_id`, or a retired field like
// `dry_run`), and silently ignoring it would hide the mistake. Reject with 400.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanRequest {
    /// The `llm_models.id` to judge with. `None` → the per-feature default
    /// (`THEME_SCAN_MODEL`, else the chat default).
    #[serde(default)]
    pub model_id: Option<String>,
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
    /// The full [`ThemeScanSummary`] once `completed`, read from the stored
    /// `summary_json` (a render convenience — the GET never re-queries Neo4j).
    /// `None` while running/failed.
    ///
    /// Each entry in its `suggestions` array is ANNOTATED on the way out with
    /// `ordinal` and `applied` (see [`ThemeScanSuggestion`]); the stored row itself
    /// is never modified. So the wire shape is `ThemeScanSummary` PLUS those two
    /// per-suggestion fields, which is how the frontend types it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<serde_json::Value>,
}

/// One row of the scan-run HISTORY list for
/// `GET .../scan-runs` — a lightweight HEADER, deliberately NOT the full result.
///
/// ## Why headers only (not `summary_json`, not verdicts)
///
/// The history list renders a compact per-run row (model, counts, timestamp) for
/// every run of a scenario — potentially many. Shipping
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
    /// The candidate pool this run read — the history table's "Candidates" column
    /// and the basis of the "+Δ since the previous scan" delta (task 1.7C, R2).
    ///
    /// `0` means the run never reached the pool read; the client renders that as an
    /// em dash, never as the number zero (Standing Rule 1: "read nothing" and
    /// "read a pool of zero" are different states).
    pub candidates_read: i32,
    /// Why a failed run failed, verbatim, so the history can say
    /// "Failed — vLLM offline" rather than just "Failed".
    ///
    /// Present-as-null (not skipped): the absence of a reason on a `completed` run
    /// is meaningful, and a client must be able to tell it from a build that does
    /// not serve the field.
    pub error: Option<String>,
    /// Whether this was a dry run (judged, nothing merged), so the row can be
    /// labelled. Measured on DEV: 3 of 4 stored runs are dry runs.
    pub dry_run: bool,
    /// How much bigger the pool this run read was than the previous measurable
    /// run's — the history's **New** column and the meta line's "+Δ since the
    /// previous scan" (task 1.7C, ruling R2).
    ///
    /// SIGNED, because after task 2.5's re-anchoring the pool can legitimately
    /// shrink and a `-12` that says so beats a `0` that hides it.
    ///
    /// `None` means there is no earlier run that recorded a pool read — the first
    /// measurable scan. Rendered as an em dash in the table and omitted from the
    /// meta line, never as `0`: "the pool did not change" and "there is nothing to
    /// compare against" are different facts.
    ///
    /// Computed at list assembly by `services::scan_run_delta::with_pool_deltas`,
    /// because it is a property of this run's POSITION in the history rather than
    /// of the run itself. A `ScanRunHeader` built outside that path carries `None`.
    pub pool_delta: Option<i32>,
    // No merge_count / last_merged_at: a per-RUN merge counter is an artifact of
    // the retired run-level merge model. Merge is now pick-keyed, so "this run was
    // merged 2×" answers a question nobody asks — the provenance signal the human
    // needs is per-FACT (the judgment strip on the card) and per-PICK (a
    // suggestion's applied state, derived from `scenario_fact_refs.source_run_id`).
    // The `scan_run_merges` audit table still records every merge event; it simply
    // has no UI derived from it.
}

/// Response for `GET .../scan-runs` — the scenario's run history, newest first.
///
/// Wrapped in `{ runs: [...] }` (rather than a bare array) to mirror the
/// `/api/scan/models` `{ models: [...] }` shape and to leave room for list-level
/// metadata later without a breaking change. Task 2.15 is the first taker: the
/// history's two control strings ride here.
#[derive(Debug, Clone, Serialize)]
pub struct ScanRunListResponse {
    pub runs: Vec<ScanRunHeader>,
    /// The words the history's own controls speak, from the settings store.
    ///
    /// Served with the list rather than fetched separately because they are
    /// useless without it, and because a second round-trip would let a row render
    /// before its controls have words — which is how a button ships blank.
    pub wording: ScanHistoryWording,
}

/// The two stored strings a scan-history row renders (task 2.15).
#[derive(Debug, Clone, Serialize)]
pub struct ScanHistoryWording {
    /// The control that reopens a run's results.
    pub view_label: String,
    /// The confirmation before a run is destroyed. Carries `{run}`, which the
    /// browser fills with the row's own when-label — the only part of this
    /// sentence the server cannot know, because the timestamp is formatted in the
    /// reader's locale.
    pub delete_confirm_template: String,
}

/// Request body for `POST …/scan-runs/:run_id/merge`.
///
/// `graph_node_ids` are the picks the human CHECKED in the results list — merge
/// writes the scan's judgment onto ONLY these (ratified Option A). An empty list is
/// rejected as a 400 by the service ([`ThemeScanError::EmptySelection`]); the
/// frontend also disables Merge until a pick is checked, so an empty body is a
/// defensive floor, not the normal path.
///
/// ## Rust Learning: `#[serde(default)]` on the field, `deny_unknown_fields` on the struct
///
/// A missing `graph_node_ids` key deserializes to an empty `Vec` rather than a 422
/// deserialization error, so a malformed/empty body reaches our OWN empty-selection
/// check and returns the domain-specific 400 ("check at least one pick") instead of a
/// generic serde rejection — a clearer, actionable message (Standing Rule 1). But a
/// MISSPELLED key (`graph_node_id` for `graph_node_ids`) is a caller mistake, not an
/// absence: `deny_unknown_fields` rejects it as a 400 rather than silently merging
/// zero — the same discipline `ScanRequest` uses, so a typo can never masquerade as
/// "nothing selected".
// serde: deny_unknown_fields — an unknown key here is a client bug (a typo'd
// `graph_node_id`), and ignoring it would let a mistake read as an empty selection.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanRunMergeRequest {
    #[serde(default)]
    pub graph_node_ids: Vec<String>,
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
/// `candidates_read == relevant + irrelevant + failed`. This identity is the
/// recall guarantee made observable (Standing Rule 1) — every candidate the scan
/// read lands in exactly one bucket, so a dropped quote would show up as a count
/// that does not add up rather than as a silent absence.
///
/// ## Domain note: a summary is a SCORECARD, not a receipt
///
/// Nothing in this struct describes a change to the case. A scan reports what the
/// model thought; the human's Merge selected is what commits any of it. Read
/// `relevant: 31` as "31 picks are waiting for your decision", never as "31 facts
/// were added".
#[derive(Debug, Clone, Serialize)]
pub struct ThemeScanSummary {
    /// The `scan_runs.run_id` this scan recorded — the handle the benchmark
    /// comparison query joins on. Present on every run (dry or not).
    pub run_id: Uuid,
    /// The `llm_models.id` this run judged with (the resolved model, after the
    /// request/`THEME_SCAN_MODEL`/chat-default fallback).
    pub model_id: String,
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
    /// Candidate quotes JUDGED — one per LLM call, so one per de-duplicated,
    /// non-excluded group (task 2.15 Tier 2).
    ///
    /// ## This number changed meaning on 2026-08-08, and the old one did not vanish
    ///
    /// It used to be the whole gathered pool, because every gathered row was
    /// judged. Now the pool is de-duplicated and pre-filtered first, so the pool
    /// size lives in [`Self::conservation`]`.pool` and this is what the judge
    /// actually saw. The identity `candidates_read == relevant + irrelevant +
    /// failed` still holds exactly; what it partitions is the judged set.
    pub candidates_read: usize,
    /// Where every gathered row went — the arithmetic that proves nothing was
    /// dropped silently (Standing Rule 1). Rendered under the run's results.
    pub conservation: ScanConservation,
    /// Verdicts judged RELEVANT to the accusation — the picks offered to the human
    /// for selection. NOTHING is persisted to `scenario_fact_refs` on their behalf;
    /// they become candidate facts only when the human merges them.
    ///
    /// (Formerly `relevant_written`, back when a scan upserted its own picks. The
    /// name outlived the write, so it was renamed rather than left describing
    /// something that no longer happens — Standing Rule 1 applies to field names
    /// as much as to logs.)
    pub relevant: usize,
    /// Verdicts judged NOT relevant to the accusation. Never suggested; a sample is
    /// surfaced in [`Self::rejected_sample`] for the honesty check.
    pub irrelevant: usize,
    /// Candidates whose verdict could not be produced: an LLM call that exhausted
    /// retries, a reply that failed the strict parse, or an out-of-set role.
    /// Counted, never silently dropped — each is logged with its `evidence_id` and
    /// cause (Standing Rule 1).
    ///
    /// A failed *write* is no longer among these causes: the scan performs no
    /// per-candidate write, so this count now means exactly "the model could not
    /// give a usable verdict" and nothing else.
    pub failed: usize,
    /// The relevant picks, so the client can render them without a second
    /// round-trip. One entry per [`Self::relevant`] — the two cannot disagree.
    pub suggestions: Vec<ThemeScanSuggestion>,
    /// A bounded, spread-out sample of the rejected quotes for the Amendment-1
    /// honesty check. Empty when nothing was rejected; at most
    /// `THEME_SCAN_REJECTED_SAMPLE_SIZE` entries otherwise.
    pub rejected_sample: Vec<ThemeScanRejected>,
}

/// Where every gathered candidate went in one run (task 2.15 Tier 2, item 1c).
///
/// ## The identity, and why it is a wire type rather than a log line
///
/// ```text
/// pool = excluded_empty + excluded_statement_type + excluded_too_short
///      + duplicates_collapsed + judged
/// ```
///
/// A scan that quietly judged 124 of a 148-row pool would otherwise look exactly
/// like a scan of a 124-row pool. These counts are what make the difference
/// visible — on screen, under the results, in the run's own record. They are
/// stored in the run summary (JSONB, so no migration) and are FROZEN there: they
/// describe what this run did, and a later settings change must not rewrite them.
///
/// Historical runs recorded before this shipped carry no `conservation` key at
/// all; the read path treats that as "not measured" and shows no line, never as
/// a pool of zero (Standing Rule 1).
#[derive(Debug, Clone, Serialize)]
pub struct ScanConservation {
    /// Rows the gather query returned — the ungated 100%-recall pool.
    pub pool: usize,
    /// Set aside: no quote text at all, or only whitespace.
    pub excluded_empty: usize,
    /// Set aside: a statement kind the settings row names as content-free.
    pub excluded_statement_type: usize,
    /// Set aside: shorter than the configured minimum, with no paired question.
    pub excluded_too_short: usize,
    /// Folded into a byte-identical twin. Each still receives its own verdict row
    /// and is covered by one merge (ruling R2) — this counts LLM calls saved, not
    /// candidates discarded.
    pub duplicates_collapsed: usize,
    /// Groups sent to the judge — one LLM call each.
    pub judged: usize,
}

/// One RELEVANT verdict — a pick offered to the human, written nowhere.
///
/// ## Two fields the WIRE carries that this struct does not
///
/// When a stored summary is served by `GET .../scan-runs/:run_id`, each suggestion
/// is annotated with `ordinal` (`number | null`) and `applied` (`bool`) by
/// [`crate::services::scan_run_enrich`]. They are absent here on purpose: neither
/// is a property of the SCAN.
///
/// * `ordinal` is the scenario's identity for the candidate, and may be assigned
///   after this run judged it;
/// * `applied` changes every time the human merges, so a value frozen into the
///   stored summary would be stale the moment it was written.
///
/// Baking either into this struct would make the historical record claim something
/// that is only true at read time. `reason` stays here because it IS the model's
/// output and never changes.
///
/// (The doc note that the `reason` is "stored in the row's `note` column" was true
/// of an earlier design; the scan writes no fact-ref row at all now. The reason is
/// persisted in `scan_run_verdicts.reason` — the audit record — and rides this
/// card. The `note` column is the human's alone.)
#[derive(Debug, Clone, Serialize)]
pub struct ThemeScanSuggestion {
    /// The Evidence node id this suggestion references (the `graph_node_id`
    /// column, equal to the candidate's `evidence_id`).
    pub graph_node_id: String,
    /// The role the judge assigned — one of the four `FactRole` tokens
    /// (`supports` / `corroborates` / `contradicts` / `rebuts`).
    pub proposed_role: String,
    /// The judge's one-to-two-sentence justification. Persisted in
    /// `scan_run_verdicts.reason` (the audit record), never on a fact ref.
    pub reason: String,
    /// The judge's self-reported confidence in `[0.0, 1.0]`. Reaches
    /// `scenario_fact_refs.confidence` only if the human merges this pick.
    pub confidence: f32,
    /// Every node id this ONE pick speaks for — `graph_node_id` first, then any
    /// byte-identical twin folded into it (task 2.15 Tier 2, item 1a).
    ///
    /// ## Why the client merges this list and not the single id
    ///
    /// The twins were never judged separately (one quote, one call), and Roman's
    /// ruling is that one ruling covers the set. So checking this pick and
    /// pressing Merge must write the judgment onto every member — otherwise the
    /// identical sentence would come back tomorrow as an unruled candidate, which
    /// is the exact duplicate work the collapse exists to end.
    ///
    /// Always at least one entry, so the client needs no special case for a pick
    /// with no twins.
    pub covers_node_ids: Vec<String>,
    /// How many pool rows this pick speaks for. `1` for an ordinary pick; `2` for
    /// one of the measured S-4 twins. Shown on the card so the human knows a
    /// single ruling is doing more than one row's work.
    pub duplicate_count: usize,
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
