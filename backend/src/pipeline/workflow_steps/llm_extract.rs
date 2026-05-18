//! Restate workflow step handlers for LLM extraction (pass 1 and pass 2).
//!
//! Wraps the clean orchestrators
//! ([`run_pass1_extraction`](crate::pipeline::steps::llm_extract::run_pass1_extraction)
//! and [`run_pass2_extraction`](crate::pipeline::steps::llm_extract_pass2::run_pass2_extraction))
//! with the Restate error classification, the
//! `documents.status = "EXTRACTED"` Postgres write after pass-1, and
//! the `run_pass2` skip path for pass-2.
//!
//! ## No fake framework objects
//!
//! After the three-refactor sequence (1/3 `call_with_rate_limit_retry`,
//! 2/3 pass-1 orchestrator, 3/3 pass-2 orchestrator), neither
//! orchestrator takes `CancellationToken` or `ProgressReporter`
//! parameters. The Restate handlers call them with `(doc_id, db,
//! context)` and consume the clean return structs — no
//! `CancellationToken::new()`, no `ProgressReporter::new(pool, Uuid::nil())`,
//! no noop-progress shim. The legacy `Step::execute` impls remain as
//! thin wrappers that add the cancel check + `set_step_result` audit
//! JSON + FSM routing on top of the same clean cores.
//!
//! ## Idempotency
//!
//! Both pass-1 and pass-2 orchestrators have their own idempotency
//! guards keyed on a COMPLETED `extraction_runs` row scoped by
//! `(document_id, pass_number)`. Re-running the Restate step
//! short-circuits inside the orchestrator with no DB writes beyond
//! the SELECT that confirms the COMPLETED row.
//!
//! The step handlers surface that short-circuit distinctly in the
//! Restate journal via the `skipped_already_complete` flag on each
//! return struct:
//!
//! - **Pass-1**: [`crate::pipeline::steps::llm_extract::Pass1ExtractionResult::skipped_already_complete`]
//!   → summary `skipped_pass1_already_complete_run_pass2=…`.
//! - **Pass-2**: [`crate::pipeline::steps::llm_extract_pass2::Pass2ExtractionResult::skipped_already_complete`]
//!   → summary `skipped_pass2_already_complete`.

use std::error::Error;
use std::sync::Arc;

use restate_sdk::errors::{HandlerError, TerminalError};
use sqlx::PgPool;

use super::{record_step_lifecycle, StepOutcome, STEP_LLM_EXTRACT_PASS1, STEP_LLM_EXTRACT_PASS2};
use crate::models::document_status::STATUS_EXTRACTED;
use crate::pipeline::config::{resolve_config, ProcessingProfile};
use crate::pipeline::context::AppContext;
use crate::pipeline::steps::llm_extract::{
    default_profile_name_from_schema, run_pass1_extraction, LlmExtractError,
};
use crate::pipeline::steps::llm_extract_pass2::run_pass2_extraction;
use crate::repositories::pipeline_repository;

/// Restate workflow step: LLM extraction pass 1.
///
/// Runs the same orchestrator the legacy worker does — chunking,
/// per-chunk LLM calls, rate-limit retry, entity merge, audit
/// snapshots — by calling the clean
/// [`run_pass1_extraction`](crate::pipeline::steps::llm_extract::run_pass1_extraction)
/// directly with `(doc_id, db, context)`. On success writes
/// `documents.status = "EXTRACTED"` (the canonical status from
/// [`crate::models::document_status`]). Returns a short summary
/// string for the Restate journal.
///
/// ## Idempotency
///
/// Inherited from the orchestrator: a COMPLETED pass-1 run for
/// `(doc_id, pass_number=1)` causes the orchestrator to return
/// without touching the LLM or the DB. The step handler detects
/// this via the [`Pass1ExtractionResult::skipped_already_complete`]
/// flag and emits a distinct "skipped" summary.
///
/// ## Error classification
///
/// All `LlmExtractError` variants are routed through
/// [`classify_llm_extract_error`] which decides terminal-vs-retryable.
/// Errors that aren't `LlmExtractError` (e.g. raw `sqlx::Error`
/// wrapped into the orchestrator's `Box<dyn Error>` return) are
/// conservatively classified as retryable — Restate will back off
/// and try again.
#[tracing::instrument(skip(app), fields(doc_id = %doc_id, step = STEP_LLM_EXTRACT_PASS1))]
pub async fn step_llm_extract_pass1(
    app: &Arc<AppContext>,
    doc_id: &str,
) -> Result<String, HandlerError> {
    record_step_lifecycle(
        &app.pipeline_pool,
        doc_id,
        STEP_LLM_EXTRACT_PASS1,
        step_llm_extract_pass1_body(app, doc_id),
    )
    .await
}

/// Body of [`step_llm_extract_pass1`]. Returns the 11-key audit JSON
/// matching `pipeline/steps/llm_extract.rs:193`. `None` fields on the
/// idempotency short-circuit path serialize to JSON `null`, which is
/// what the legacy `progress.set_step_result(...)` also emits — the
/// shape stays byte-identical regardless of whether work was done.
#[tracing::instrument(skip(app), fields(doc_id = %doc_id))]
async fn step_llm_extract_pass1_body(
    app: &Arc<AppContext>,
    doc_id: &str,
) -> Result<StepOutcome, HandlerError> {
    // After Refactor 2/3, run_pass1_extraction has a clean signature
    // — no CancellationToken, no ProgressReporter, no make_noop_progress
    // shim. The legacy Worker's `Step::execute` impl wraps this same
    // function with its own cancel check + progress.set_step_result.
    let result = run_pass1_extraction(doc_id, &app.pipeline_pool, app.as_ref())
        .await
        .map_err(|e| classify_dyn_llm_error(doc_id, "llm_extract_pass1", e))?;

    // Postgres status write — mirrors the Restate state write the
    // workflow performs after this step. Decision #3 in P2-2b: pass-1
    // success → "EXTRACTED"; pass-2 success → no status write (the
    // value stays "EXTRACTED" until Verify writes "VERIFIED" in
    // P2-2c). Note: when this invocation hit the idempotency
    // short-circuit, the documents.status row might already be
    // "EXTRACTED" — the UPDATE is still a no-op-equivalent write
    // (one row, identical value).
    pipeline_repository::update_document_status(&app.pipeline_pool, doc_id, STATUS_EXTRACTED)
        .await
        .map_err(|e| match e {
            pipeline_repository::PipelineRepoError::NotFound(_) => TerminalError::new(format!(
                "step_llm_extract_pass1: documents row for '{doc_id}' \
                 disappeared while updating status. Cannot proceed; \
                 confirm the document still exists in the documents table."
            ))
            .into(),
            other => HandlerError::from(format!(
                "step_llm_extract_pass1: failed to update status for \
                 '{doc_id}': {other}. Will retry."
            )),
        })?;

    let summary = if result.skipped_already_complete {
        format!(
            "skipped_pass1_already_complete_run_pass2={}",
            result.run_pass2
        )
    } else {
        format!(
            "pass1_complete entities={} relationships={} input_tokens={} output_tokens={} run_pass2={}",
            result.entity_count.unwrap_or(0),
            result.relationship_count.unwrap_or(0),
            result.input_tokens.unwrap_or(0),
            result.output_tokens.unwrap_or(0),
            result.run_pass2,
        )
    };

    tracing::info!(
        doc_id = %doc_id,
        skipped = result.skipped_already_complete,
        entity_count = ?result.entity_count,
        relationship_count = ?result.relationship_count,
        input_tokens = ?result.input_tokens,
        output_tokens = ?result.output_tokens,
        run_pass2 = result.run_pass2,
        "step_llm_extract_pass1: complete"
    );
    // Audit JSON shape matches `pipeline/steps/llm_extract.rs:193`.
    // See [`build_pass1_result_summary`] for the byte-identical
    // mapping the legacy worker established.
    Ok(StepOutcome {
        summary,
        result_summary: build_pass1_result_summary(&result),
        // `skipped_early` mirrors the orchestrator's idempotency
        // flag: when pass-1 short-circuited on a COMPLETED run, the
        // body's wall-clock work was bookkeeping only (the `SELECT`
        // probing the COMPLETED row plus the `UPDATE` writing the
        // `EXTRACTED` status). Reporting duration_secs = 0.0 in that
        // case keeps the Execution History panel's "skipped"
        // semantics consistent across all 8 steps (extract_text's
        // idempotency-guard skip and pass-2's profile-driven skip
        // both set this same flag — see the StepOutcome docstring
        // for the contract).
        skipped_early: result.skipped_already_complete,
    })
}

/// Build the 11-key `result_summary` JSON for pass-1, matching
/// `pipeline/steps/llm_extract.rs:193` byte-for-byte.
///
/// Extracted from the inline call site for two reasons: (1) so the
/// JSON shape can be unit-tested without standing up a database, and
/// (2) so a single-line change to a key name is mechanically isolated
/// from the surrounding lifecycle code. The audit-trail contract this
/// JSON encodes is consumed by external tooling that the Rust tests
/// cannot see — without a unit test pinning the keys, a rename of a
/// `Pass1ExtractionResult` field would silently break the contract.
fn build_pass1_result_summary(
    result: &crate::pipeline::steps::llm_extract::Pass1ExtractionResult,
) -> serde_json::Value {
    serde_json::json!({
        "entity_count": result.entity_count,
        "relationship_count": result.relationship_count,
        "input_tokens": result.input_tokens,
        "output_tokens": result.output_tokens,
        "chunk_count": result.chunk_count,
        "chunks_succeeded": result.chunks_succeeded,
        "chunks_failed": result.chunks_failed,
        "profile": result.profile,
        "model": result.model,
        "chunking_mode": result.chunking_mode,
        "system_prompt_file": result.system_prompt_file,
    })
}

/// Restate workflow step: LLM extraction pass 2 (relationships).
///
/// The Restate workflow body calls every step unconditionally — pass-2
/// is no exception. This handler checks the resolved profile's
/// `run_pass2` flag itself and short-circuits when false, since the
/// legacy worker FSM is gone (no `next_step_after_pass1` routing on
/// the Restate path).
///
/// On the `run_pass2 = true` path, delegates to the existing
/// [`run_pass2_extraction`](crate::pipeline::steps::llm_extract_pass2::run_pass2_extraction)
/// orchestrator. No documents.status write here per decision #3 —
/// `STATUS_EXTRACTED` stays in place until Verify writes
/// `STATUS_VERIFIED` in P2-2c.
///
/// ## Idempotency
///
/// Two distinct skip paths produce distinguishable journal summaries:
///
/// 1. **Profile says no:** `resolve_run_pass2` returns false →
///    `"skipped_pass2_not_configured"`.
/// 2. **Already complete:** the orchestrator's returned
///    [`Pass2ExtractionResult::skipped_already_complete`] flag is
///    `true` → `"skipped_pass2_already_complete"`. After Refactor
///    3/3 this signal lives on the struct itself, so the
///    workflow-layer probe that used to do a pre-call SELECT was
///    deleted — one fewer DB roundtrip per pass-2 invocation.
#[tracing::instrument(skip(app), fields(doc_id = %doc_id, step = STEP_LLM_EXTRACT_PASS2))]
pub async fn step_llm_extract_pass2(
    app: &Arc<AppContext>,
    doc_id: &str,
) -> Result<String, HandlerError> {
    record_step_lifecycle(
        &app.pipeline_pool,
        doc_id,
        STEP_LLM_EXTRACT_PASS2,
        step_llm_extract_pass2_body(app, doc_id),
    )
    .await
}

/// Body of [`step_llm_extract_pass2`]. Three outcome shapes:
///
/// - **Profile says no** (`run_pass2 = false`): `skipped_early =
///   true`, `result_summary` is `{"skipped": true, "reason":
///   "run_pass2_not_configured"}`. The core orchestrator is never
///   called, so the `pipeline_steps.duration_secs` is recorded as
///   0.0.
/// - **Already complete** or **success**: `skipped_early = false`,
///   `result_summary` is the 9-key shape from
///   `pipeline/steps/llm_extract_pass2.rs:112`. On the
///   already-complete path the count fields are zero and the string
///   fields are `null`, matching what the legacy `set_step_result`
///   would emit for the same `Pass2ExtractionResult`.
#[tracing::instrument(skip(app), fields(doc_id = %doc_id))]
async fn step_llm_extract_pass2_body(
    app: &Arc<AppContext>,
    doc_id: &str,
) -> Result<StepOutcome, HandlerError> {
    // [1] Profile-driven skip. If the resolved profile has
    //     run_pass2=false, the workflow's unconditional call is
    //     satisfied without running pass-2.
    let run_pass2 = resolve_run_pass2(&app.pipeline_pool, app.as_ref(), doc_id).await?;
    if !run_pass2 {
        tracing::info!(
            doc_id = %doc_id,
            "step_llm_extract_pass2: profile has run_pass2=false, skipping"
        );
        // Distinct shape from the post-orchestrator paths — `"skipped":
        // true` lets audit tooling distinguish "we never ran pass-2 for
        // this doc" from "we ran it and got zero relationships."
        return Ok(StepOutcome {
            summary: "skipped_pass2_not_configured".to_string(),
            result_summary: build_pass2_not_configured_summary(),
            skipped_early: true,
        });
    }

    // [2] Call the clean orchestrator. After Refactor 3/3 the
    //     orchestrator no longer takes a CancellationToken or
    //     ProgressReporter — it returns a Pass2ExtractionResult that
    //     carries the already-complete signal directly.
    let result = run_pass2_extraction(doc_id, &app.pipeline_pool, app.as_ref())
        .await
        .map_err(|e| classify_dyn_llm_error(doc_id, "llm_extract_pass2", e))?;

    let summary = if result.skipped_already_complete {
        tracing::info!(
            doc_id = %doc_id,
            "step_llm_extract_pass2: COMPLETED pass-2 extraction_run exists, skipping"
        );
        "skipped_pass2_already_complete".to_string()
    } else {
        let s = format!("pass2_complete relationships={}", result.relationship_count);
        tracing::info!(
            doc_id = %doc_id,
            relationship_count = result.relationship_count,
            local_entities = result.local_entities,
            cross_doc_entities = result.cross_doc_entities,
            input_tokens = result.input_tokens,
            output_tokens = result.output_tokens,
            "step_llm_extract_pass2: complete"
        );
        s
    };

    // Audit JSON shape matches `pipeline/steps/llm_extract_pass2.rs:112`.
    // See [`build_pass2_result_summary`] for the byte-identical
    // mapping. On the already-complete path the count fields are
    // zero and the string fields are `None` (→ JSON `null`) — same
    // shape, just different content.
    Ok(StepOutcome {
        summary,
        result_summary: build_pass2_result_summary(&result),
        // See pass-1's identical comment above — `skipped_early`
        // mirrors the orchestrator's idempotency flag so all 8
        // step types report skip duration consistently.
        skipped_early: result.skipped_already_complete,
    })
}

/// Build the 9-key `result_summary` JSON for pass-2 (success and
/// already-complete paths), matching
/// `pipeline/steps/llm_extract_pass2.rs:112` byte-for-byte. The
/// `pass: 2` literal mirrors the legacy emit. Extracted for
/// testability — see [`build_pass1_result_summary`] for the same
/// rationale.
fn build_pass2_result_summary(
    result: &crate::pipeline::steps::llm_extract_pass2::Pass2ExtractionResult,
) -> serde_json::Value {
    serde_json::json!({
        "pass": 2,
        "relationship_count": result.relationship_count,
        "local_entities": result.local_entities,
        "cross_doc_entities": result.cross_doc_entities,
        "input_tokens": result.input_tokens,
        "output_tokens": result.output_tokens,
        "profile": result.profile,
        "model": result.model,
        "pass2_template_file": result.pass2_template_file,
    })
}

/// Build the skipped-not-configured `result_summary` JSON for
/// pass-2 when the resolved profile has `run_pass2 = false`. The
/// `"reason": "run_pass2_not_configured"` sentinel distinguishes
/// this from pass-2's other skip path (already-complete), where
/// the full 9-key shape is emitted instead.
fn build_pass2_not_configured_summary() -> serde_json::Value {
    serde_json::json!({
        "skipped": true,
        "reason": "run_pass2_not_configured",
    })
}

/// Resolve the `run_pass2` flag for the document's profile.
///
/// Pass-2 in the legacy FSM was skipped via `next_step_after_pass1`
/// when `resolved.run_pass2 = false`. The Restate workflow body has
/// no FSM — it calls `step_llm_extract_pass2` unconditionally — so
/// the skip must happen inside the handler. This helper re-resolves
/// the config to read the flag (the legacy orchestrators don't
/// expose it as a return value). The reads here are cheap: two
/// `pipeline_config` row reads + one YAML file open.
///
/// Returns retryable error on transient DB failure; terminal on
/// missing config or unloadable profile (matches the classification
/// the orchestrator would have applied internally).
async fn resolve_run_pass2(
    pool: &PgPool,
    context: &AppContext,
    doc_id: &str,
) -> Result<bool, HandlerError> {
    let pipe_config = pipeline_repository::get_pipeline_config(pool, doc_id)
        .await
        .map_err(|e| {
            HandlerError::from(format!(
                "step_llm_extract_pass2: transient failure reading pipeline_config \
                 for '{doc_id}': {e}. Will retry."
            ))
        })?
        .ok_or_else(|| {
            HandlerError::from(TerminalError::new(format!(
                "step_llm_extract_pass2: no pipeline_config row for '{doc_id}'. \
                 Confirm upload completed and the config-creation step ran."
            )))
        })?;

    let overrides = pipeline_repository::get_pipeline_config_overrides(pool, doc_id)
        .await
        .map_err(|e| {
            HandlerError::from(format!(
                "step_llm_extract_pass2: transient failure reading pipeline_config \
                 overrides for '{doc_id}': {e}. Will retry."
            ))
        })?;

    let profile_name = overrides
        .profile_name
        .clone()
        .unwrap_or_else(|| default_profile_name_from_schema(&pipe_config.schema_file));

    let profile =
        ProcessingProfile::load(context.registry.profile_dir(), &profile_name).map_err(|e| {
            HandlerError::from(TerminalError::new(format!(
                "step_llm_extract_pass2: failed to load profile '{profile_name}' \
                 for '{doc_id}': {e}. Fix the profile YAML and redeploy before retry."
            )))
        })?;

    let resolved = resolve_config(&profile, &overrides);
    Ok(resolved.run_pass2)
}

// ── Error classification ────────────────────────────────────────

/// Downcast a `Box<dyn Error>` from the legacy orchestrators into
/// `LlmExtractError` and route through [`classify_llm_extract_error`].
///
/// The orchestrators (`run_llm_extract`, `run_pass2_extraction`)
/// return `Result<_, Box<dyn Error + Send + Sync>>` — the underlying
/// type is `LlmExtractError` in the typed-error paths but may be a
/// `sqlx::Error` or other concrete type in a few transitional spots.
/// We downcast for typed classification, and fall back to retryable
/// for anything we can't downcast — Restate will retry transient
/// failures of any shape.
fn classify_dyn_llm_error(
    doc_id: &str,
    step_name: &'static str,
    e: Box<dyn Error + Send + Sync>,
) -> HandlerError {
    match e.downcast::<LlmExtractError>() {
        Ok(typed) => classify_llm_extract_error(doc_id, step_name, &typed),
        Err(boxed) => HandlerError::from(format!(
            "step_{step_name}: unclassified failure for '{doc_id}': {boxed}. \
             Will retry."
        )),
    }
}

/// Classify an [`LlmExtractError`] as terminal or retryable for
/// Restate.
///
/// Mirrors the P2-2a `classify_extract_error` pattern. Decision
/// rules:
///
/// - Permanent configuration / state issues → terminal. The retry
///   will see the same state and fail the same way.
/// - Transient infrastructure (LLM timeout, DB timeout, semaphore
///   closed) → retryable. Restate's exponential backoff likely
///   resolves these.
/// - LLM output bugs (non-JSON response after retries, serialization
///   failures) → terminal. These indicate template/prompt drift or
///   a programming bug that needs operator intervention.
///
/// ## Rust Learning: pattern-match on enum reference
///
/// `match e { Variant => ... }` where `e: &LlmExtractError` lets us
/// classify without consuming the error — useful because the caller
/// already owns `*typed: LlmExtractError` and we want to keep the
/// Display impl available for the message body via the `{e}` inside
/// each format!.
fn classify_llm_extract_error(
    doc_id: &str,
    step_name: &'static str,
    e: &LlmExtractError,
) -> HandlerError {
    use LlmExtractError as E;
    match e {
        // ── Terminal: configuration / state issues ─────────────
        E::DocumentNotFound { .. } => TerminalError::new(format!(
            "step_{step_name}: document '{doc_id}' not found in database. \
             Confirm the upload completed before invoking the workflow."
        ))
        .into(),
        E::NoPipelineConfig { .. } => TerminalError::new(format!(
            "step_{step_name}: no pipeline_config row for document '{doc_id}'. \
             Confirm the config-creation step ran after upload."
        ))
        .into(),
        E::SchemaLoadFailed { schema_file, .. } => TerminalError::new(format!(
            "step_{step_name}: failed to load schema '{schema_file}' for \
             '{doc_id}'. {e}. Fix the schema file and redeploy."
        ))
        .into(),
        E::ProfileLoadFailed { .. } => TerminalError::new(format!(
            "step_{step_name}: profile load failed for '{doc_id}'. {e}. \
             Fix the profile YAML and redeploy."
        ))
        .into(),
        E::ModelNotFound { model_id } => TerminalError::new(format!(
            "step_{step_name}: model '{model_id}' not found or inactive for \
             '{doc_id}'. Activate the model in the llm_models table or pick \
             another model in the profile."
        ))
        .into(),
        E::ProviderConstructionFailed { .. } => TerminalError::new(format!(
            "step_{step_name}: LLM provider construction failed for '{doc_id}'. \
             {e}. Check ANTHROPIC_API_KEY / LLM_PROVIDER env vars and redeploy."
        ))
        .into(),
        E::NoPass2Template { profile_name } => TerminalError::new(format!(
            "step_{step_name}: profile '{profile_name}' has run_pass2=true but \
             no pass2_template_file for '{doc_id}'. Either set run_pass2=false \
             in the profile or add a pass2_template_file entry."
        ))
        .into(),
        E::NoCompletedPass1 { .. } => TerminalError::new(format!(
            "step_{step_name}: no COMPLETED pass-1 extraction_run for \
             '{doc_id}'. Pass-1 must succeed before pass-2 can run."
        ))
        .into(),
        E::NoTextPages { .. } => TerminalError::new(format!(
            "step_{step_name}: document '{doc_id}' has no text pages. \
             Re-run extract_text or confirm the document has extractable \
             content."
        ))
        .into(),
        E::PromptBuildFailed { .. } => TerminalError::new(format!(
            "step_{step_name}: prompt assembly failed for '{doc_id}'. {e}. \
             Fix the template and redeploy."
        ))
        .into(),

        // ── Terminal: LLM output bugs ────────────────────────────
        E::ResponseNotJson { preview, .. } => TerminalError::new(format!(
            "step_{step_name}: LLM returned non-JSON response for '{doc_id}'. \
             {e}. Preview: {preview}. Check extraction_runs.raw_output and \
             investigate template prompt or model output drift."
        ))
        .into(),
        E::EntitySerializationFailed { .. } | E::RelationshipSerializationFailed { .. } => {
            TerminalError::new(format!(
                "step_{step_name}: re-serialization of merged extraction \
                 output failed for '{doc_id}'. {e}. This indicates a \
                 programming bug — investigate the merged entity/relationship \
                 shape (likely a NaN float or non-serializable type)."
            ))
            .into()
        }

        // ── Terminal: operator-initiated cancellation ────────────
        //
        // Mirrors the Restate SDK's own
        // `CancelSignalReceived → TerminalError(409, "cancelled")`
        // mapping at `restate-sdk-0.6.0/src/endpoint/context.rs:884`.
        // MUST be terminal — a Retryable classification here would
        // bounce the cancelled invocation through Restate's retry
        // loop and undo the whole point of polling
        // `documents.is_cancelled` in the chunk loop.
        E::Cancelled { .. } => TerminalError::new(format!(
            "step_{step_name}: {e}. The cooperative-cancellation \
             poller observed `documents.is_cancelled = true` and \
             short-circuited before the next Anthropic API call. No \
             retry — the operator explicitly asked to stop."
        ))
        .into(),

        // ── Retryable: transient infrastructure ──────────────────
        E::LlmCallFailed { .. }
        | E::SemaphoreClosed
        | E::InsertRunFailed { .. }
        | E::CompleteRunFailed { .. }
        | E::StoreFailed { .. } => HandlerError::from(format!(
            "step_{step_name}: transient failure for '{doc_id}'. {e}. \
             Will retry."
        )),
    }
}

// ─────────────────────────────────────────────────────────────────
// Unit tests
//
// Same pattern as P2-2a's `classify_extract_error` tests: one test
// per `LlmExtractError` variant, asserting terminal-vs-retryable
// through the SDK's Display impl ("Terminal error" vs "Retryable
// error" prefix on `HandlerError::as_ref()`).
// ─────────────────────────────────────────────────────────────────

// Unit tests for the classify functions and the result-summary
// builders live in `llm_extract_tests.rs` (kept out-of-line to stay
// closer to the 300-line module-size budget; matches the
// `pipeline/registry.rs` / `registry_tests.rs` and
// `extract_text.rs` / `extract_text_tests.rs` idioms).
#[cfg(test)]
#[path = "llm_extract_tests.rs"]
mod tests;
