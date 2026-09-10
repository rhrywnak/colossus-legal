//! The Theme Scan ERROR TAXONOMY — every condition under which a whole scan
//! cannot proceed, and the HTTP surface each one earns.
//!
//! ## Why this is its own module
//!
//! It was the top third of `theme_scan.rs`, which owns the shared vocabulary of a
//! scan (the phase structs, the case-fenced loader) AND `prepare_scan`. Task
//! SCAN_SERVER_STATE added four variants and pushed that file past the 300-line
//! limit, and the taxonomy is the piece with the cleanest seam: it is what every
//! phase RETURNS, referenced by all four scan modules and by the route, while
//! nothing in it knows anything about how a scan is prepared.
//!
//! `theme_scan.rs` re-exports [`ThemeScanError`], so every existing
//! `crate::services::theme_scan::ThemeScanError` import is unchanged — the type
//! moved, its address did not.
//!
//! The one place the mapping from these variants to HTTP statuses lives is
//! `api::scenario_theme_scan::map_scan_error`, and it has a test per arm. A new
//! variant added here without an arm there falls into the catch-all 500, which is
//! why that module's tests pin each one by name.

use uuid::Uuid;

use crate::bias::repository::BiasRepositoryError;
use crate::domain::llm_params::LlmConfigError;
use crate::repositories::pipeline_repository::PipelineRepoError;

/// Top-level, scan-aborting failures.
///
/// These are distinct from per-item verdict failures (a bad LLM reply for one
/// quote), which are COUNTED in the summary rather than returned here. Every
/// variant is a condition under which the whole scan cannot meaningfully proceed.
/// The route handler maps each to an HTTP status.
///
/// ## Rust Learning: `#[source]` on a wrapped cause
///
/// `#[source]` exposes the underlying error in the chain so `{source}` in the
/// message and a structured logger both see the real cause (Standing Rule 1: the
/// failure names *what* failed and *why*), without this enum re-stringifying it.
#[derive(Debug, thiserror::Error)]
pub enum ThemeScanError {
    /// The scenario row could not be read (DB/connection error).
    #[error("failed to load scenario {scenario_id}: {source}")]
    ScenarioLoadFailed {
        scenario_id: Uuid,
        #[source]
        source: PipelineRepoError,
    },

    /// No scenario with that id in that case (absent, or the case-fence rejected
    /// a cross-case id). Same observable for both — a caller must not learn that
    /// an id exists in another case.
    #[error("scenario {scenario_id} not found in case '{case_slug}'")]
    ScenarioNotFound {
        case_slug: String,
        scenario_id: Uuid,
    },

    /// The stored `definition` jsonb did not parse as a `ScenarioDefinition`
    /// (e.g. a retired v1 shape). Loud, not defaulted.
    ///
    /// The recovery action rides the message because this now fails BEFORE the
    /// run row exists (the 400 split), so the toast is the only surface it has.
    #[error(
        "scenario {scenario_id} has a definition this build cannot parse: {source} \
         — re-open the scenario and re-save it to rewrite the definition in the \
         shape this build reads"
    )]
    DefinitionInvalid {
        scenario_id: Uuid,
        #[source]
        source: serde_json::Error,
    },

    /// The scenario has NEITHER attack text nor a legacy meaning. A scan needs
    /// judgment criteria; this is a user-fixable precondition, surfaced clearly
    /// rather than scanning with empty criteria.
    ///
    /// The variant keeps its .389 name so callers and tests that match on it are
    /// unaffected, but the condition widened with the one-attack-box ruling: it
    /// now fires only when BOTH texts are blank. The message names the box the
    /// human can actually see and fill.
    #[error(
        "scenario {scenario_id} has no attack text — a scan needs judgment \
         criteria; write what they claim on the scenario's identity before scanning"
    )]
    EmptyAttackMeaning { scenario_id: Uuid },

    /// A delete was requested for a run the RECORD depends on. Refused as a 409
    /// rather than performed, because deleting the run would destroy both
    /// provenance records at once — the `scan_run_merges` events cascade away and
    /// every `scenario_fact_refs.source_run_id` pointing at it nulls out — leaving
    /// the human's rulings in the case with no trace of what put those candidates
    /// in front of them.
    ///
    /// ## Domain note: what "cited" means since the projection (architect ruling R1)
    ///
    /// Under the retired merge model this fired for a run somebody had merged.
    /// Under the projection there are no new merge events, and the count that
    /// matters is the second one: how many RULINGS name this run as the thing that
    /// proposed them. A run one ruling has drawn on is part of the ledger's chain
    /// of custody and stays undeletable; a junk scan nobody ruled from has neither
    /// count above zero and deletes freely, taking its unruled proposals with it.
    /// That is the case R-d exists for, and it still works.
    ///
    /// The message says so in plain words, because a human meeting this 409 is
    /// mid-cleanup and needs to know it is a rule rather than a fault.
    #[error(
        "scan run {run_id} is part of the record — {attributed_facts} ruling(s) \
         cite it as the scan that proposed them, and {merge_events} historical \
         merge event(s) reference it. Its provenance is kept on purpose, so the \
         run cannot be deleted. Rulings you have already made are unaffected."
    )]
    ScanRunCited {
        run_id: Uuid,
        merge_events: i64,
        attributed_facts: i64,
    },

    /// The pre-delete provenance check itself failed. Kept distinct from a
    /// successful check: an unreadable check must never be treated as "no
    /// provenance, go ahead and delete" (Standing Rule 1).
    #[error("failed to check merge provenance for run {run_id} before deletion: {source}")]
    ScanRunProvenanceCheckFailed {
        run_id: Uuid,
        #[source]
        source: PipelineRepoError,
    },

    /// The scenario's definition names no `target`, so there is nobody to scan.
    ///
    /// ## Why the sibling `SubjectResolveFailed` variant is gone (2026-08-07)
    ///
    /// Until this date, resolution could also fail at the GRAPH layer, because a
    /// target-less scenario fell back to looking up the case-default subject —
    /// and that fallback is what let a scenario scan and gather over a subject
    /// nobody chose (see `services::scenario_subject`). With the fallback
    /// removed, resolution reads one field off a parsed definition: it cannot
    /// touch the graph, so it cannot fail at the graph, and a variant for a
    /// failure that can no longer happen would be a message no operator will
    /// ever see and every reader has to reason about.
    ///
    /// The message names the human fix (author a target), not a config key: this
    /// is now a scenario-authoring state, not a deployment misconfiguration.
    #[error(
        "scenario {scenario_id}: no subject to scan — the scenario names no target. \
         Edit the scenario's identity and name who it is about, then scan again"
    )]
    SubjectUnresolvable { scenario_id: Uuid },

    /// Reading the candidate quote set for the subject failed.
    #[error("failed to read candidate evidence for subject '{subject_id}': {source}")]
    CandidateReadFailed {
        subject_id: String,
        #[source]
        source: BiasRepositoryError,
    },

    /// The configured prompt file is missing/unreadable. Fail-loud, naming the
    /// path (mirrors the extraction template load).
    ///
    /// Since task 2.15 the filename is the `theme_scan_prompt_file` SETTINGS ROW,
    /// and both realistic triggers point at the same two fixes: the row names a
    /// file nobody deployed, or the file was removed after the row was set. The
    /// message names both — and deliberately no longer names the retired
    /// `THEME_SCAN_PROMPT_FILE` env var, which would send an operator hunting for
    /// something this build does not read.
    ///
    /// Reaching this at all means the two guards were passed: the write path
    /// refuses a filename that does not resolve, and boot refuses to start when
    /// the stored one has stopped resolving. So the realistic path here is a file
    /// that vanished while the service was running.
    #[error(
        "Theme Scan prompt file not readable at '{path}': {source} \
         — deploy the file to the template directory, or correct the \
         theme_scan_prompt_file row on the Settings page"
    )]
    PromptFileMissing {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Looking up the selected model row failed at the database layer.
    ///
    /// Toast-only since the 400 split (it fails before the run row exists), so the
    /// message carries the recovery action itself.
    #[error(
        "failed to load model '{model_id}': {source} \
         — the model registry could not be read; verify the database is reachable"
    )]
    ModelLookupFailed {
        model_id: String,
        #[source]
        source: sqlx::Error,
    },

    /// The selected model id is not an active `llm_models` row. User-fixable
    /// (pick a model that exists and is active) → the route maps it to 400.
    #[error(
        "model '{model_id}' is not an active registered model — pick a model that \
         exists and is active"
    )]
    ModelNotAvailable { model_id: String },

    /// The model's parameters could not be resolved/constrained (a corrupt row
    /// value, or a task request the model cannot satisfy). Names the model and
    /// carries the resolver's own typed cause.
    ///
    /// Both recovery actions ride the message because the cause decides which one
    /// applies, and the caller can see the difference in `{source}` but the code
    /// cannot: a task the model cannot satisfy is fixed by picking another model,
    /// a corrupt stored value by correcting the row.
    #[error(
        "model '{model_id}' has invalid LLM parameters: {source} \
         — pick another model, or correct that model's row in llm_models"
    )]
    ParamsInvalid {
        model_id: String,
        #[source]
        source: LlmConfigError,
    },

    /// Constructing the provider from the model row failed (e.g. a vLLM row with
    /// no endpoint). Carries the builder's message.
    #[error("failed to build a provider for model '{model_id}': {detail}")]
    ProviderBuildFailed { model_id: String, detail: String },

    /// HARD GATE: the selected vLLM endpoint did not answer `/v1/models`. The
    /// scan REFUSES rather than dispatch to an unknown/unreachable model. 503.
    #[error(
        "vLLM endpoint '{endpoint}' is unreachable for the model gate: {detail} \
         — verify the vLLM service is running and serving at that endpoint, or \
         correct the model's api_endpoint in the llm_models table"
    )]
    VllmUnreachable { endpoint: String, detail: String },

    /// HARD GATE: the vLLM endpoint answered, but the loaded model is not the one
    /// selected — naming BOTH so the operator knows exactly what to switch. 503.
    #[error(
        "vLLM endpoint '{endpoint}' has the wrong model loaded: selected '{selected}' \
         but loaded '{loaded}' — switch the vLLM model or pick the loaded one"
    )]
    VllmModelMismatch {
        endpoint: String,
        selected: String,
        loaded: String,
    },

    /// Writing the `scan_runs` row at the start of a background scan failed —
    /// either the stub INSERT or the promote UPDATE. The job cannot be tracked, so
    /// the POST fails rather than spawning an untracked task. Server-side (500).
    #[error("failed to record the start of scan run {run_id}: {source}")]
    ScanRunWriteFailed {
        run_id: Uuid,
        #[source]
        source: PipelineRepoError,
    },

    /// The promote-to-`running` UPDATE matched zero rows: the stub row this scan
    /// just wrote is no longer in its birth state (deleted, or already moved on).
    /// Distinct from [`Self::ScanRunWriteFailed`] — the write itself SUCCEEDED and
    /// simply found nothing to promote, which is a different diagnosis. Refused
    /// rather than continued: judging would spend real LLM budget on a run whose
    /// progress and outcome nothing can report. Server-side (500).
    #[error(
        "scan run {run_id} could not be promoted to running — its start record is \
         gone or already terminal, so the scan was not launched"
    )]
    ScanRunNotPromotable { run_id: Uuid },

    /// Reading a `scan_runs` row for the poll failed (DB error). Server-side (500).
    #[error("failed to read scan run {run_id}: {source}")]
    ScanRunReadFailed {
        run_id: Uuid,
        #[source]
        source: PipelineRepoError,
    },

    /// No scan run with that id in this scenario/case (absent, or the case/scenario
    /// fence rejected it). Same observable for both — a caller must not learn that
    /// a run exists elsewhere. The route maps it to 404.
    #[error("scan run {run_id} not found")]
    ScanRunNotFound { run_id: Uuid },

    /// Listing a scenario's scan-run history failed (DB error). Distinct from
    /// [`Self::ScanRunReadFailed`] — that names a single `run_id`; this names the
    /// `scenario_id` whose history could not be read (Standing Rule 1 — the error
    /// says WHAT failed, not a fabricated run handle). Server-side (500).
    #[error("failed to list scan runs for scenario {scenario_id}: {source}")]
    ScanRunListFailed {
        scenario_id: Uuid,
        #[source]
        source: PipelineRepoError,
    },

    /// A second scan was requested for a scenario that already has one running.
    ///
    /// Answered with a 409 carrying the RUNNING run's id, so the browser adopts
    /// that run and shows its progress instead of showing an error for a state
    /// that is not a fault — the human asked for a scan of this scenario and there
    /// is one; it simply is not the one they just clicked.
    ///
    /// ## Domain note: why refusing is the kind thing to do
    ///
    /// A scan is one metered LLM call per candidate over a pool that can run to
    /// several hundred. Two concurrent runs on one scenario spend twice the budget
    /// to answer the same question, write two competing verdict sets, and race each
    /// other's progress bumps on a row neither owns — and until this refusal the
    /// only thing standing between a human and that was a button that went grey in
    /// one browser tab.
    ///
    /// Raised from two places that agree: the pre-check in `theme_scan_start`
    /// (which is what runs in practice) and the `scan_runs_one_running_per_scenario`
    /// index (which catches two POSTs landing in the same millisecond).
    #[error(
        "a scan of scenario {scenario_id} is already running (run {run_id}) — \
         watch that one, or stop it before starting another"
    )]
    ScanAlreadyRunning { scenario_id: Uuid, run_id: Uuid },

    /// The in-flight check itself failed (DB error) before a scan could start.
    ///
    /// Kept distinct from a successful check, exactly as
    /// [`Self::ScanRunProvenanceCheckFailed`] is for the delete: an unreadable
    /// check must never be treated as "nothing is running, go ahead". That would
    /// fail in the expensive direction — a second background job judging a pool
    /// the first one is already paying for (Standing Rule 1).
    #[error(
        "failed to check for a running scan on scenario {scenario_id}: {source} \
         — the scan was not started because the check could not be read; verify \
         the pipeline database is reachable and try again"
    )]
    ScanRunInFlightCheckFailed {
        scenario_id: Uuid,
        #[source]
        source: PipelineRepoError,
    },

    /// A stop was requested for a run that is not `running`.
    ///
    /// Carries the status it actually holds, because that is the whole answer: a
    /// run that completed while the human's finger was moving needs no stopping,
    /// and one already `cancelled` was stopped by somebody else. Neither is a
    /// fault; both are a 409 that says which.
    #[error("scan run {run_id} is not running — it is '{status}', so there is nothing to stop")]
    ScanRunNotRunning { run_id: Uuid, status: String },

    /// The run's row says `running` but no live judging task in THIS process owns
    /// it, so there is no token to cancel and nothing that will ever write
    /// `cancelled`.
    ///
    /// Reachable only in a narrow window — the judging loop has finished and
    /// removed its token while the finalize write is still in flight — or if a
    /// `running` row somehow outlived its task without the startup sweep seeing
    /// it. Reported rather than answered with a cheerful 202, because a 202 would
    /// promise a stop that nothing is going to perform (Standing Rule 1: "I
    /// stopped it" and "nothing here can stop it" are different observables).
    #[error(
        "scan run {run_id} is recorded as running but no judging task in this \
         backend owns it — it is finishing, or its task did not survive; reload \
         the page to see the state it settles in"
    )]
    ScanRunNotCancellable { run_id: Uuid },

    /// Deleting one scan run failed (DB error). Distinct from
    /// [`Self::ScanRunNotFound`] — that is a legitimate "no such run here" (zero
    /// rows deleted → 404); this is an actual DB failure the delete could not
    /// even attempt cleanly (Standing Rule 1 — the two outcomes are not
    /// collapsed). Names the `run_id` it could not delete. Server-side (500).
    #[error("failed to delete scan run {run_id}: {source}")]
    ScanRunDeleteFailed {
        run_id: Uuid,
        #[source]
        source: PipelineRepoError,
    },
}

#[cfg(test)]
#[path = "theme_scan_error_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "theme_scan_state_error_tests.rs"]
mod state_tests;
