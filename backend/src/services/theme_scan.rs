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
use crate::repositories::pipeline_repository::{get_scenario, PipelineRepoError, ScenarioRecord};
use crate::services::theme_scan_provider::ResolvedScanProvider;
use crate::services::vllm_model_gate::{assert_vllm_model_loaded, VllmGateError};
use crate::state::AppState;

// CONST: the verdict token budget is a fixed protocol shape, not a deployment
// knob. A verdict is a tiny four-key JSON object; 512 is a generous ceiling that
// would only ever change if the verdict SHAPE changes — and that is a code change
// (the `Verdict` struct + the prompt shipped together), never per-environment
// tuning. Roman pinned this as a named constant (no env) in the D2b decision. It
// is `pub` because `theme_scan_provider::scan_task_spec` reads it as the scan's
// TASK-layer `max_tokens`, so the resolver and the verdict cap agree from one
// source of truth (Chunk B).
pub const THEME_SCAN_MAX_TOKENS: u32 = 512;

// Why no `const THEME_SCAN_PROMPT` here anymore: the prompt FILENAME (its
// version) was a compiled-in const, so bumping the prompt version meant a
// rebuild+deploy — a Standing-Rule-2 violation (config that varies across
// deployments must be editable via env/YAML + restart). It now comes from
// `AppConfig::theme_scan_prompt_file` (env `THEME_SCAN_PROMPT_FILE`, default
// `theme_scan_prompt_v2.md`), resolved in `config.rs`. The resolved filename is
// carried on `PreparedScan.prompt_file` and recorded per-run into
// `scan_runs.resolved_params` — which is what actually satisfies the
// "which prompt judged this run" provenance concern the const only pretended to.
// The directory the filename resolves against was already env-driven via the
// registry's `template_path` (unchanged read path). The read itself now lives in
// [`load_scan_prompt`], called at the very start of a scan — see its doc.

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

    /// The scenario has no `attack_meaning`. A scan needs judgment criteria; this
    /// is a user-fixable precondition, surfaced clearly rather than scanning with
    /// empty criteria.
    #[error(
        "scenario {scenario_id} has no attack_meaning — a scan needs judgment \
         criteria; author the accusation meaning before scanning"
    )]
    EmptyAttackMeaning { scenario_id: Uuid },

    /// A merge was requested with no picks checked. User-fixable (check at least
    /// one pick) → 400; kept distinct from a run that merges zero because it HAS no
    /// relevant picks, so the two look different to the caller (Standing Rule 1).
    #[error("no picks selected to merge from run {run_id} — check at least one pick, then Merge")]
    EmptySelection { run_id: Uuid },

    /// A delete was requested for a run whose judgments are already part of the
    /// case. Refused as a 409 rather than performed, because deleting the run would
    /// destroy both provenance records at once — the `scan_run_merges` events
    /// cascade away and every `scenario_fact_refs.source_run_id` pointing at it
    /// nulls out — leaving merged judgments in the case with no trace of their
    /// origin. Unmerged runs remain deletable, so this never blocks junk-scan
    /// cleanup. The counts ride the message so the human knows what is holding it.
    #[error(
        "scan run {run_id} has been merged ({merge_events} merge event(s), \
         {attributed_facts} candidate fact(s) still credit it) — its provenance is \
         retained and the run cannot be deleted"
    )]
    ScanRunMerged {
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

    /// Resolving the case-default subject failed at the graph layer.
    ///
    /// Distinct from [`Self::SubjectUnresolvable`], which is the scenario naming
    /// nobody: this is the lookup itself failing. Like [`Self::DefinitionInvalid`]
    /// it now fails before the run row exists, so the message carries its own
    /// recovery action rather than relying on Run History to be read later.
    #[error(
        "failed to resolve the default subject for scenario {scenario_id}: {source} \
         — verify the graph is reachable, or that CASE_DEFAULT_SUBJECT_NAME names a \
         party this case actually has"
    )]
    SubjectResolveFailed {
        scenario_id: Uuid,
        #[source]
        source: BiasRepositoryError,
    },

    /// Neither the scenario definition's `target` nor a configured case-default
    /// subject yielded a subject to scan.
    #[error(
        "scenario {scenario_id}: no subject to scan — the scenario names no target \
         and no case-default subject is configured (CASE_DEFAULT_SUBJECT_NAME)"
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
    /// path (mirrors the extraction template load). Now that the filename is
    /// `THEME_SCAN_PROMPT_FILE` config, the realistic trigger is a misconfigured
    /// env var or an un-deployed asset — so the message names the recovery action.
    #[error(
        "Theme Scan prompt file not readable at '{path}': {source} \
         — deploy the file to the registry's template dir or correct THEME_SCAN_PROMPT_FILE"
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

    /// Merging one scan run's relevant picks into the scenario failed (DB error).
    /// Distinct from [`Self::ScanRunNotFound`] (the run is absent / not in this
    /// scenario → 404) and from a legitimate zero-count merge (the run has no
    /// relevant picks, or every pick was preserved as human curation → 200 with
    /// `merged = 0`). This is an actual write failure. Server-side (500).
    #[error("failed to merge scan run {run_id}: {source}")]
    ScanRunMergeFailed {
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
    pub(crate) attack_meaning: Arc<str>,
    pub(crate) scan_prompt: Arc<str>,
    pub(crate) provider: Arc<dyn LlmProvider>,
    /// The resolved+constrained parameters (drive the wire max_tokens AND the
    /// `scan_runs` snapshot).
    pub(crate) params: ResolvedLlmParams,
    /// The resolved model id (after request/`THEME_SCAN_MODEL`/chat-default).
    pub(crate) model_id: String,
    /// The resolved prompt filename this run judged with (from
    /// `THEME_SCAN_PROMPT_FILE`, default `theme_scan_prompt_v2.md`). Carried so
    /// it can be recorded into `scan_runs.resolved_params` — the run→prompt
    /// provenance that was previously only implied by the compiled-in const.
    pub(crate) prompt_file: String,
    /// Per-run fan-out cap (A5: model `max_concurrency`, else env default).
    pub(crate) concurrency: usize,
    pub(crate) cost_per_input_token: Option<f64>,
    pub(crate) cost_per_output_token: Option<f64>,
    pub(crate) candidates: Vec<BiasInstance>,
}

/// The judging prompt, read from disk before a scan is allowed to start.
///
/// Carries the FILENAME alongside the text because the filename is the run's
/// prompt provenance (recorded into `scan_runs.resolved_params`), and the text is
/// what the judge actually sends.
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
    pub(crate) attack_meaning: String,
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
    validated: ValidatedScan,
    prompt: ScanPrompt,
) -> Result<PreparedScan, ThemeScanError> {
    let ValidatedScan {
        attack_meaning,
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

    Ok(PreparedScan {
        attack_meaning: Arc::from(attack_meaning),
        scan_prompt: Arc::from(prompt.text),
        provider: resolved.provider,
        params: resolved.params,
        model_id: resolved.model_id,
        prompt_file: prompt.file,
        concurrency: resolved.concurrency,
        cost_per_input_token: resolved.cost_per_input_token,
        cost_per_output_token: resolved.cost_per_output_token,
        candidates,
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
