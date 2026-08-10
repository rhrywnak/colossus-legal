//! Theme Scan service (D2b) — batched LLM judgment of candidate quotes.
//!
//! A scenario's `definition` carries an `attack_meaning` (plain-English prose,
//! authored in D1). Theme Scan:
//!
//! 1. reads EVERY candidate quote about the scenario's subject
//!    (`all_evidence_about_subject` — ungated, so recall is 100% by
//!    construction: nothing is pre-filtered by keyword or embedding);
//! 2. asks the deterministic LLM judge to rate each quote against the
//!    `attack_meaning`, returning `{relevant, proposed_role, reason, confidence}`;
//! 3. writes each RELEVANT verdict to `scenario_fact_refs` as an `undecided`
//!    suggestion (idempotent per-row upsert), awaiting a human include/drop
//!    ruling; irrelevant verdicts are counted and sampled but never persisted;
//! 4. returns a [`ThemeScanSummary`] with the counts, the written suggestions,
//!    and a rejected sample for the honesty check.
//!
//! This module owns the SHARED VOCABULARY of a scan — the error taxonomy every
//! phase returns, the structs that cross phase boundaries (`ScanPrompt`,
//! `ValidatedScan`, `PreparedScan`), the case-fenced scenario loader — plus
//! `prepare_scan`, the phase that does the work a validated request implies.
//! (Those are `pub(crate)`, so they are named here rather than linked; a doc
//! link from this public module page to a private item is a rustdoc warning.)
//!
//! The rest of the start path is split by phase, because the phase boundary is
//! also the boundary of what a failure leaves behind:
//!
//! * [`crate::services::theme_scan_validate`] — the checks whose failure is the
//!   CALLER'S to fix. Answered with a 4xx and no run row.
//! * `prepare_scan` (here) and [`crate::services::theme_scan_start`] — the vLLM
//!   gate, the candidate read, the run record, and the judging task. A failure
//!   from here on is recorded on the run row.
//!
//! The per-quote judging and result-persistence helpers live in the sibling
//! [`crate::services::theme_scan_judge`], and the verdict parser in
//! [`crate::services::theme_scan_parse`] — kept apart so no single file exceeds
//! the module-size limit and each piece is independently testable.
//!
//! ## Concurrency (D2b STEP-1 decision)
//!
//! Candidates are judged concurrently with `buffer_unordered`, each call bounded
//! by [`AppState::theme_scan_semaphore`] — a DEDICATED cap, not the pipeline's
//! `llm_semaphore`, so a scan and document extraction never starve each other.
//! The provider is `Send + Sync + 'static` with no interior mutability and each
//! `call_with_rate_limit_retry` owns its own retry loop, so concurrent calls are
//! safe; the retry wrapper absorbs any rate-limit brush from the fan-out.

use std::sync::Arc;

use colossus_extract::LlmProvider;
use uuid::Uuid;

use crate::bias::dto::BiasInstance;
use crate::bias::repository::{BiasRepository, BiasRepositoryError};
use crate::domain::llm_params::{LlmConfigError, ResolvedLlmParams};
use crate::dto::theme_scan::ScanConservation;
use crate::repositories::pipeline_repository::{get_scenario, PipelineRepoError, ScenarioRecord};
use crate::services::theme_scan_prefilter::{
    log_prefilter, prepare_pool, CandidateGroup, PrefilterConfig,
};
use crate::services::theme_scan_provider::ResolvedScanProvider;
use crate::services::vllm_model_gate::{assert_vllm_model_loaded, VllmGateError};
use crate::state::AppState;

// Why there is no `const THEME_SCAN_MAX_TOKENS` here anymore.
//
// It was `pub const THEME_SCAN_MAX_TOKENS: u32 = 512`, and the argument for
// pinning it — Roman's D2b decision — was this: the verdict token budget is a
// fixed PROTOCOL SHAPE, not a deployment knob. A verdict is a tiny four-key JSON
// object, so 512 was a generous ceiling that would only ever move if the verdict
// SHAPE moved, and that is a code change (the `Verdict` struct and the prompt ship
// together), never per-environment tuning.
//
// That argument was correct, and it stayed correct for exactly as long as its
// premise held: that the model's OUTPUT is the verdict. Claude Opus 5 runs
// adaptive thinking by default, and `max_tokens` caps thinking and answer
// TOGETHER — so the budget stopped being "room for a four-key object" and became
// "room for a four-key object AND however much the model decides to think first".
//
// Measured 2026-08-09 (CC_REPORT_BAKEOFF_SCORECARD.md), S-4 run `2c7b7d87`: 7 of
// 104 judged groups failed. Six replies were cut off mid-word inside the `reason`
// string — all of them while writing `"relevant": true` — and the seventh emitted
// no text block at all, having spent the whole budget thinking. The tell was
// counter-intuitive: the FAILED replies were shorter (101–328 chars) than the
// successful ones (377 average), because the loss was upstream of the text.
//
// So the cap is now the `theme_scan_max_tokens` SETTINGS ROW, read at scan start
// beside the prompt pointer, asserted at boot, and editable with no rebuild — the
// same journey the prompt made above, for the same reason: a value that decides
// whether a verdict survives is not a protocol constant just because it once
// looked like one. `constrain` still clamps it to each model's own
// `max_output_tokens`, which is a different job and unchanged.

// Why no `const THEME_SCAN_PROMPT` here anymore, in two moves.
//
// It began as a compiled-in const, so bumping the prompt version meant a
// rebuild+deploy — a Standing-Rule-2 violation. It then became the env var
// `THEME_SCAN_PROMPT_FILE` with a compiled default, which fixed the rebuild and
// left the value invisible: measured on DEV, the var was never set, so a constant
// nobody could see still decided which prompt judged every scan.
//
// Since task 2.15 it is the `theme_scan_prompt_file` SETTINGS ROW — visible on the
// Settings page, editable with no restart, asserted at boot, and refused at write
// time if it names a file that is not deployed. There is no env var and no
// compiled default left to fall back to.
//
// The resolved filename is carried on `PreparedScan.prompt_file` and recorded
// per-run into `scan_runs.resolved_params`, which is what actually satisfies the
// "which prompt judged this run" provenance concern the const only pretended to.
// The directory it resolves against is still the registry's env-driven template
// dir. The read lives in `load_scan_prompt`, called at the very start of a scan.

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

/// Everything a scan needs to judge, resolved and validated up front.
///
/// Bundling these into one struct lets [`run_theme_scan`] read as a short
/// orchestration (prepare → judge → persist) while [`prepare_scan`] owns the
/// multi-step precondition checks.
pub(crate) struct PreparedScan {
    /// The text every candidate is judged against.
    ///
    /// Named for what it IS rather than where it came from (task R2). It was
    /// `attack_meaning` until .391, and after the one-attack-box ruling that name
    /// became a lie in the normal case: the value now comes from
    /// `definition.attack_text`, and only falls back to the legacy
    /// `attack_meaning` on a scenario authored before the ruling. A downstream
    /// reader seeing `attack_meaning` would have believed it was reading a gloss.
    /// `theme_scan_validate` logs which field answered.
    pub(crate) scan_criteria: Arc<str>,
    pub(crate) scan_prompt: Arc<str>,
    pub(crate) provider: Arc<dyn LlmProvider>,
    /// The resolved+constrained parameters (drive the wire max_tokens AND the
    /// `scan_runs` snapshot).
    pub(crate) params: ResolvedLlmParams,
    /// The resolved model id (after request/`THEME_SCAN_MODEL`/chat-default).
    pub(crate) model_id: String,
    /// The prompt filename this run judged with, resolved from the
    /// `theme_scan_prompt_file` settings row at scan start. Carried so it can be
    /// recorded into `scan_runs.resolved_params` — the run→prompt provenance that
    /// was previously only implied by a compiled-in const, and that matters more
    /// now that the value can change between two runs without a deploy.
    pub(crate) prompt_file: String,
    /// Per-run fan-out cap (A5: model `max_concurrency`, else env default).
    pub(crate) concurrency: usize,
    pub(crate) cost_per_input_token: Option<f64>,
    pub(crate) cost_per_output_token: Option<f64>,
    /// What the judge will see: one group per LLM call, byte-identical twins
    /// already folded together (task 2.15 Tier 2). A group of one is the ordinary
    /// case and is not special-cased anywhere downstream.
    pub(crate) groups: Vec<CandidateGroup>,
    /// The pre-filter settings THIS run was started with, frozen into
    /// `scan_runs.resolved_params` beside the LLM parameters.
    ///
    /// ## Why the settings and not just their effect
    ///
    /// The conservation block records what the pre-filter DID (15 quotes set aside
    /// for length); this records the threshold that produced it. Without it, an
    /// operator comparing two runs a week apart cannot tell a prompt change from a
    /// settings change — the numbers moved and nothing says which dial turned.
    /// Same argument as `prompt_file`, and the same reason `resolved_params` is a
    /// snapshot rather than a pointer at the mutable row (design 5.9).
    pub(crate) prefilter: PrefilterSnapshot,
    /// pool → excluded → collapsed → judged, frozen into the run's summary.
    pub(crate) conservation: ScanConservation,
}

/// The pre-filter parameters one run used, as recorded in its snapshot.
///
/// Owned `String`s rather than borrows: this outlives the settings snapshot it was
/// read from (the run record must stay readable after the row is edited), which is
/// exactly the difference between a snapshot and a reference.
pub(crate) struct PrefilterSnapshot {
    pub(crate) min_chars: usize,
    pub(crate) statement_types: Vec<String>,
}

/// The judging prompt, read from disk before a scan is allowed to start.
///
/// Carries the FILENAME alongside the text because the filename is the run's
/// prompt provenance (recorded into `scan_runs.resolved_params`), and the text is
/// what the judge actually sends.
// Debug so a test can `expect_err` on the read (which formats the Ok side).
#[derive(Debug)]
pub(crate) struct ScanPrompt {
    pub(crate) file: String,
    pub(crate) text: String,
}

/// Everything a scan needs that is decided from the REQUEST and the scenario row
/// alone — the answers [`validate_scan_request`] produces.
///
/// `subject_id` is resolved here rather than inside the candidate read because
/// "this scenario names nobody to scan about" is a question about the request,
/// while "the graph would not answer" is a question about the system. They belong
/// on opposite sides of the run record (see [`validate_scan_request`]).
pub(crate) struct ValidatedScan {
    /// The judging criteria — see [`PreparedScan::scan_criteria`] for why this is
    /// not called `attack_meaning` any more.
    pub(crate) scan_criteria: String,
    pub(crate) subject_id: String,
    pub(crate) resolved: ResolvedScanProvider,
}

/// Do the work that a validated request implies: clear the vLLM hard gate and
/// read the candidate pool. Every failure here is a typed, scan-aborting
/// [`ThemeScanError`] — and, unlike [`validate_scan_request`]'s, one that lands
/// on a run row the caller has already recorded.
///
/// Takes the already-read `prompt` rather than reading it: the prompt check must
/// happen before ANYTHING is recorded (see [`load_scan_prompt`]). See
/// `theme_scan_start::start_theme_scan` for the full order and why it matters.
pub(crate) async fn prepare_scan(
    state: &AppState,
    scenario_id: Uuid,
    validated: ValidatedScan,
    prompt: ScanPrompt,
) -> Result<PreparedScan, ThemeScanError> {
    let ValidatedScan {
        scan_criteria,
        subject_id,
        resolved,
    } = validated;

    // HARD GATE (vLLM only): before any candidate is dispatched, confirm the
    // endpoint is reachable and serving the SELECTED model. The Anthropic path
    // has `vllm_endpoint == None` and skips this. Fail-fast, before any spend.
    if let Some(endpoint) = &resolved.vllm_endpoint {
        assert_vllm_model_loaded(&state.http_client, endpoint, &resolved.model_id)
            .await
            .map_err(gate_error_into_scan_error)?;
    }

    let candidates = read_candidates(state, &subject_id).await?;

    // Tier 2: de-duplicate and pre-filter BEFORE any call is dispatched. Nothing
    // is discarded — see `theme_scan_prefilter` for the conservation identity that
    // every count below has to satisfy.
    let settings = state.settings.current();
    let prepared = prepare_pool(
        candidates,
        PrefilterConfig {
            min_chars: settings.theme_scan_prefilter_min_chars,
            dropped_statement_types: &settings.theme_scan_prefilter_statement_types,
        },
    );
    log_prefilter(scenario_id, &prepared);

    Ok(PreparedScan {
        scan_criteria: Arc::from(scan_criteria),
        scan_prompt: Arc::from(prompt.text),
        provider: resolved.provider,
        params: resolved.params,
        model_id: resolved.model_id,
        prompt_file: prompt.file,
        concurrency: resolved.concurrency,
        cost_per_input_token: resolved.cost_per_input_token,
        cost_per_output_token: resolved.cost_per_output_token,
        groups: prepared.groups,
        prefilter: PrefilterSnapshot {
            min_chars: settings.theme_scan_prefilter_min_chars,
            statement_types: settings.theme_scan_prefilter_statement_types.clone(),
        },
        conservation: prepared.conservation,
    })
}

/// Map the reusable gate's domain-agnostic [`VllmGateError`] into this service's
/// error taxonomy. The gate stays reusable (no legal-app types); the scan owns the
/// HTTP-status and recovery-message policy, so it translates at this boundary.
fn gate_error_into_scan_error(e: VllmGateError) -> ThemeScanError {
    match e {
        VllmGateError::Unreachable { endpoint, detail } => {
            ThemeScanError::VllmUnreachable { endpoint, detail }
        }
        VllmGateError::Mismatch {
            endpoint,
            selected,
            loaded,
        } => ThemeScanError::VllmModelMismatch {
            endpoint,
            selected,
            loaded,
        },
    }
}

/// Read every candidate quote about the subject (the ungated
/// `all_evidence_about_subject` set — the 100%-recall input to the judge).
async fn read_candidates(
    state: &AppState,
    subject_id: &str,
) -> Result<Vec<BiasInstance>, ThemeScanError> {
    let repo = BiasRepository::new(state.graph.clone());
    repo.all_evidence_about_subject(subject_id)
        .await
        .map_err(|source| ThemeScanError::CandidateReadFailed {
            subject_id: subject_id.to_string(),
            source,
        })
}

/// Load one scenario, enforcing the case-isolation fence.
///
/// `get_scenario` is keyed on the globally-unique `scenario_id` alone, so the
/// case-fence is applied here: a row from a different case is reported as
/// `ScenarioNotFound`, identical to a truly-absent id (a caller must not learn
/// that an id exists elsewhere).
pub(crate) async fn load_scenario_fenced(
    pool: &sqlx::PgPool,
    case_slug: &str,
    scenario_id: Uuid,
) -> Result<ScenarioRecord, ThemeScanError> {
    let record = get_scenario(pool, scenario_id)
        .await
        .map_err(|source| ThemeScanError::ScenarioLoadFailed {
            scenario_id,
            source,
        })?
        .ok_or_else(|| ThemeScanError::ScenarioNotFound {
            case_slug: case_slug.to_string(),
            scenario_id,
        })?;

    if record.case_slug != case_slug {
        tracing::warn!(
            actual_case = %record.case_slug,
            requested_case = %case_slug,
            %scenario_id,
            "theme scan: scenario requested through the wrong case path"
        );
        return Err(ThemeScanError::ScenarioNotFound {
            case_slug: case_slug.to_string(),
            scenario_id,
        });
    }
    Ok(record)
}

#[cfg(test)]
#[path = "theme_scan_tests.rs"]
mod tests;
