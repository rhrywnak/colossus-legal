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
use crate::bias::repository::BiasRepository;
use crate::domain::llm_params::ResolvedLlmParams;
use crate::dto::theme_scan::ScanConservation;
use crate::repositories::pipeline_repository::{get_scenario, ScenarioRecord};
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

// The error taxonomy moved to the sibling `theme_scan_error` when this module
// reached the 300-line limit (task SCAN_SERVER_STATE). It is RE-EXPORTED below,
// so `crate::services::theme_scan::ThemeScanError` still resolves and no caller
// changed. That module's doc says why the seam is there.
pub use crate::services::theme_scan_error::ThemeScanError;

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

// The Display tests moved with the taxonomy — see `theme_scan_error_tests.rs`,
// attached to the module that now owns the enum.
