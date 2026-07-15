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
//! This module owns the *orchestration* (load, validate, resolve, drive, log).
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
use std::time::Instant;

use chrono::Utc;
use colossus_extract::LlmProvider;
use uuid::Uuid;

use crate::bias::dto::BiasInstance;
use crate::bias::repository::{BiasRepository, BiasRepositoryError};
use crate::domain::llm_params::{LlmConfigError, ResolvedLlmParams};
use crate::dto::scenario_crud::ScenarioDefinition;
use crate::dto::ThemeScanSummary;
use crate::repositories::pipeline_repository::{get_scenario, PipelineRepoError, ScenarioRecord};
use crate::services::scenario_subject::{resolve_scenario_subject, SubjectResolveError};
use crate::services::theme_scan_judge::judge_all;
use crate::services::theme_scan_persist::{persist_and_summarize, ScanRunMeta};
use crate::services::theme_scan_provider::resolve_scan_provider;
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

// CONST: the prompt VERSION this build judges with is a code decision, not a
// deployment setting. The file CONTENT is tunable without a rebuild (edit + scp
// to the registry's template dir); selecting a *different* version is a
// deliberate, reviewed bump (`_v1` → `_v2`) that ships with any matching
// `Verdict`/parse changes, so the version token is pinned in code on purpose.
// Only the filename is compiled in; the directory it resolves against is
// env-driven via the registry (Standing Rule 2 satisfied for the path).
const THEME_SCAN_PROMPT: &str = "theme_scan_prompt_v1.md";

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
    #[error("scenario {scenario_id} has a definition this build cannot parse: {source}")]
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

    /// Resolving the case-default subject failed at the graph layer.
    #[error("failed to resolve the default subject for scenario {scenario_id}: {source}")]
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

    /// The versioned prompt file is missing/unreadable. Fail-loud, naming the
    /// path (mirrors the extraction template load).
    #[error("Theme Scan prompt file not readable at '{path}': {source}")]
    PromptFileMissing {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Looking up the selected model row failed at the database layer.
    #[error("failed to load model '{model_id}': {source}")]
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
    #[error("model '{model_id}' has invalid LLM parameters: {source}")]
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
}

/// Everything a scan needs to judge, resolved and validated up front.
///
/// Bundling these into one struct lets [`run_theme_scan`] read as a short
/// orchestration (prepare → judge → persist) while [`prepare_scan`] owns the
/// multi-step precondition checks.
struct PreparedScan {
    attack_meaning: Arc<str>,
    scan_prompt: Arc<str>,
    provider: Arc<dyn LlmProvider>,
    /// The resolved+constrained parameters (drive the wire max_tokens AND the
    /// `scan_runs` snapshot).
    params: ResolvedLlmParams,
    /// The resolved model id (after request/`THEME_SCAN_MODEL`/chat-default).
    model_id: String,
    /// Per-run fan-out cap (A5: model `max_concurrency`, else env default).
    concurrency: usize,
    cost_per_input_token: Option<f64>,
    cost_per_output_token: Option<f64>,
    candidates: Vec<BiasInstance>,
}

/// Run a Theme Scan for one scenario and return its summary.
///
/// See the module docs for the full shape. Takes `&AppState` because a scan
/// touches four subsystems (pipeline pool, graph, registry, the scan provider +
/// semaphore); it is a domain service, not a reusable pipeline step, so the
/// `AppContext`-only rule does not apply.
pub async fn run_theme_scan(
    state: &AppState,
    case_slug: &str,
    scenario_id: Uuid,
    requested_model_id: Option<String>,
    dry_run: bool,
) -> Result<ThemeScanSummary, ThemeScanError> {
    let prepared =
        prepare_scan(state, case_slug, scenario_id, requested_model_id.as_deref()).await?;
    tracing::info!(
        case_slug,
        %scenario_id,
        model_id = %prepared.model_id,
        dry_run,
        concurrency = prepared.concurrency,
        candidates = prepared.candidates.len(),
        "theme scan: judging candidates"
    );

    let summary = judge_and_record(state, scenario_id, dry_run, prepared).await;
    tracing::info!(
        case_slug,
        %scenario_id,
        run_id = %summary.run_id,
        dry_run,
        candidates_read = summary.candidates_read,
        relevant_written = summary.relevant_written,
        irrelevant = summary.irrelevant,
        failed = summary.failed,
        "theme scan: complete"
    );
    Ok(summary)
}

/// Judge the prepared candidates and record the run.
///
/// Owns the timing (`Instant` for monotonic elapsed, `Utc::now()` for the
/// wall-clock start), the `judge_all` fan-out, and the `scan_runs` metadata
/// assembly + persistence. Split from [`run_theme_scan`] to keep each under the
/// function-size limit; takes `prepared` by value since judging consumes it.
async fn judge_and_record(
    state: &AppState,
    scenario_id: Uuid,
    dry_run: bool,
    prepared: PreparedScan,
) -> ThemeScanSummary {
    let started_at = Utc::now();
    let clock = Instant::now();
    let results = judge_all(
        Arc::clone(&prepared.provider),
        Arc::clone(&state.theme_scan_semaphore),
        prepared.concurrency,
        Arc::clone(&prepared.scan_prompt),
        Arc::clone(&prepared.attack_meaning),
        prepared.params,
        prepared.candidates,
    )
    .await;
    // millis fit i64 for any real scan; the impossible overflow caps rather than
    // wrapping (Standing Rule 1).
    let duration_ms = i64::try_from(clock.elapsed().as_millis()).unwrap_or(i64::MAX);

    let meta = ScanRunMeta {
        run_id: Uuid::new_v4(),
        scenario_id,
        model_id: prepared.model_id,
        resolved_params: prepared.params,
        dry_run,
        cost_per_input_token: prepared.cost_per_input_token,
        cost_per_output_token: prepared.cost_per_output_token,
        started_at,
        duration_ms,
    };
    persist_and_summarize(&state.pipeline_pool, meta, results).await
}

/// Load the scenario, validate its preconditions, and gather the inputs a scan
/// needs: the judgment criterion, the candidate quotes, the provider, and the
/// prompt. Every failure here is a typed, scan-aborting [`ThemeScanError`].
async fn prepare_scan(
    state: &AppState,
    case_slug: &str,
    scenario_id: Uuid,
    requested_model_id: Option<&str>,
) -> Result<PreparedScan, ThemeScanError> {
    let record = load_scenario_fenced(&state.pipeline_pool, case_slug, scenario_id).await?;

    let definition: ScenarioDefinition =
        serde_json::from_value(record.definition).map_err(|source| {
            ThemeScanError::DefinitionInvalid {
                scenario_id,
                source,
            }
        })?;

    // A scan with no judgment criteria is meaningless — reject the precondition.
    let attack_meaning = definition
        .attack_meaning
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(ThemeScanError::EmptyAttackMeaning { scenario_id })?
        .to_string();

    let candidates = read_candidates(state, &definition, scenario_id).await?;

    // Per-run provider: resolve the model id → row → params → provider via the
    // unified seam (Chunk B), replacing the removed boot-time `theme_scan_provider`.
    let resolved = resolve_scan_provider(state, requested_model_id).await?;

    // HARD GATE (vLLM only): before any candidate is dispatched, confirm the
    // endpoint is reachable and serving the SELECTED model. The Anthropic path
    // has `vllm_endpoint == None` and skips this. Fail-fast, before any spend.
    if let Some(endpoint) = &resolved.vllm_endpoint {
        assert_vllm_model_loaded(&state.http_client, endpoint, &resolved.model_id)
            .await
            .map_err(gate_error_into_scan_error)?;
    }

    let prompt_path = state.registry.template_path(THEME_SCAN_PROMPT);
    let scan_prompt = std::fs::read_to_string(&prompt_path).map_err(|source| {
        ThemeScanError::PromptFileMissing {
            path: prompt_path,
            source,
        }
    })?;

    Ok(PreparedScan {
        attack_meaning: Arc::from(attack_meaning),
        scan_prompt: Arc::from(scan_prompt),
        provider: resolved.provider,
        params: resolved.params,
        model_id: resolved.model_id,
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

/// Resolve the scan subject and read every candidate quote about it (the ungated
/// `all_evidence_about_subject` set — the 100%-recall input to the judge).
///
/// Subject resolution is delegated to the shared
/// [`crate::services::scenario_subject::resolve_scenario_subject`] so the scan
/// and the 1a.2 gather endpoint read the SAME subject pool by construction (see
/// that module's docs). The shared resolver's own error is mapped back into the
/// scan's existing [`ThemeScanError`] variants here — the scan's error surface
/// is unchanged; only where those variants are *constructed* moved.
async fn read_candidates(
    state: &AppState,
    definition: &ScenarioDefinition,
    scenario_id: Uuid,
) -> Result<Vec<BiasInstance>, ThemeScanError> {
    let subject_id = resolve_scenario_subject(state, definition)
        .await
        .map_err(|e| match e {
            SubjectResolveError::DefaultLookupFailed { source } => {
                ThemeScanError::SubjectResolveFailed {
                    scenario_id,
                    source,
                }
            }
            SubjectResolveError::Unresolvable => {
                ThemeScanError::SubjectUnresolvable { scenario_id }
            }
        })?;
    tracing::debug!(%scenario_id, subject_id = %subject_id, "theme scan: subject resolved");

    let repo = BiasRepository::new(state.graph.clone());
    repo.all_evidence_about_subject(&subject_id)
        .await
        .map_err(|source| ThemeScanError::CandidateReadFailed { subject_id, source })
}

/// Load one scenario, enforcing the case-isolation fence.
///
/// `get_scenario` is keyed on the globally-unique `scenario_id` alone, so the
/// case-fence is applied here: a row from a different case is reported as
/// `ScenarioNotFound`, identical to a truly-absent id (a caller must not learn
/// that an id exists elsewhere).
async fn load_scenario_fenced(
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
