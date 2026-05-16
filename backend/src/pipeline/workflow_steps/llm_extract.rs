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
#[tracing::instrument(skip(app), fields(doc_id = %doc_id, step = "llm_extract_pass1"))]
pub async fn step_llm_extract_pass1(
    app: &Arc<AppContext>,
    doc_id: &str,
) -> Result<String, HandlerError> {
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
    Ok(summary)
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
#[tracing::instrument(skip(app), fields(doc_id = %doc_id, step = "llm_extract_pass2"))]
pub async fn step_llm_extract_pass2(
    app: &Arc<AppContext>,
    doc_id: &str,
) -> Result<String, HandlerError> {
    // [1] Profile-driven skip. If the resolved profile has
    //     run_pass2=false, the workflow's unconditional call is
    //     satisfied without running pass-2.
    let run_pass2 = resolve_run_pass2(&app.pipeline_pool, app.as_ref(), doc_id).await?;
    if !run_pass2 {
        tracing::info!(
            doc_id = %doc_id,
            "step_llm_extract_pass2: profile has run_pass2=false, skipping"
        );
        return Ok("skipped_pass2_not_configured".to_string());
    }

    // [2] Call the clean orchestrator. After Refactor 3/3 the
    //     orchestrator no longer takes a CancellationToken or
    //     ProgressReporter — it returns a Pass2ExtractionResult that
    //     carries the already-complete signal directly.
    let result = run_pass2_extraction(doc_id, &app.pipeline_pool, app.as_ref())
        .await
        .map_err(|e| classify_dyn_llm_error(doc_id, "llm_extract_pass2", e))?;

    if result.skipped_already_complete {
        tracing::info!(
            doc_id = %doc_id,
            "step_llm_extract_pass2: COMPLETED pass-2 extraction_run exists, skipping"
        );
        return Ok("skipped_pass2_already_complete".to_string());
    }

    let summary = format!("pass2_complete relationships={}", result.relationship_count);
    tracing::info!(
        doc_id = %doc_id,
        relationship_count = result.relationship_count,
        local_entities = result.local_entities,
        cross_doc_entities = result.cross_doc_entities,
        input_tokens = result.input_tokens,
        output_tokens = result.output_tokens,
        "step_llm_extract_pass2: complete"
    );
    Ok(summary)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns `true` when `e` is the Terminal branch of HandlerError.
    fn display_message(e: &HandlerError) -> String {
        let inner: &dyn Error = e.as_ref();
        format!("{inner}")
    }

    fn is_terminal(e: &HandlerError) -> bool {
        display_message(e).starts_with("Terminal error")
    }

    // ── Terminal variants ───────────────────────────────────────

    #[test]
    fn classify_document_not_found_is_terminal() {
        let err = LlmExtractError::DocumentNotFound {
            document_id: "doc-x".into(),
        };
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
        assert!(is_terminal(&c), "DocumentNotFound must be terminal");
        let msg = display_message(&c);
        assert!(msg.contains("doc-x"), "msg must name doc_id: {msg}");
        assert!(
            msg.contains("upload completed"),
            "msg must hint recovery: {msg}"
        );
    }

    #[test]
    fn classify_no_pipeline_config_is_terminal() {
        let err = LlmExtractError::NoPipelineConfig {
            document_id: "doc-x".into(),
        };
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(
            msg.contains("config-creation"),
            "msg must point at config step: {msg}"
        );
    }

    #[test]
    fn classify_profile_load_failed_is_terminal() {
        let err = LlmExtractError::ProfileLoadFailed {
            message: "Profile file not found: /etc/profiles/missing.yaml".into(),
        };
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(
            msg.contains("profile YAML"),
            "msg must mention profile YAML: {msg}"
        );
        assert!(msg.contains("redeploy"), "msg must hint deploy: {msg}");
    }

    #[test]
    fn classify_model_not_found_is_terminal() {
        let err = LlmExtractError::ModelNotFound {
            model_id: "claude-deprecated".into(),
        };
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(
            msg.contains("claude-deprecated"),
            "msg must name model: {msg}"
        );
        assert!(
            msg.contains("llm_models"),
            "msg must point at the table: {msg}"
        );
    }

    #[test]
    fn classify_provider_construction_failed_is_terminal() {
        let err = LlmExtractError::ProviderConstructionFailed {
            message: "ANTHROPIC_API_KEY unset".into(),
        };
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(
            msg.contains("ANTHROPIC_API_KEY") || msg.contains("LLM_PROVIDER"),
            "msg must name the env vars to check: {msg}"
        );
    }

    #[test]
    fn classify_no_pass2_template_is_terminal() {
        let err = LlmExtractError::NoPass2Template {
            profile_name: "no_pass2_template_profile".into(),
        };
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass2", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(
            msg.contains("no_pass2_template_profile"),
            "msg must name the profile: {msg}"
        );
        assert!(
            msg.contains("run_pass2"),
            "msg must mention run_pass2: {msg}"
        );
    }

    #[test]
    fn classify_no_completed_pass1_is_terminal() {
        let err = LlmExtractError::NoCompletedPass1 {
            document_id: "doc-x".into(),
        };
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass2", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(
            msg.contains("Pass-1"),
            "msg must mention pass-1 prerequisite: {msg}"
        );
    }

    #[test]
    fn classify_no_text_pages_is_terminal() {
        let err = LlmExtractError::NoTextPages {
            document_id: "doc-x".into(),
        };
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(
            msg.contains("extract_text"),
            "msg must point at extract_text: {msg}"
        );
    }

    #[test]
    fn classify_schema_load_failed_is_terminal() {
        // Use a real PipelineError construction path via from_file on
        // a missing file. The construction details aren't critical to
        // the classification — we just need the variant.
        // Simulate it: build via the source error's Display being the
        // important part for the message.
        // We'll construct with a minimal stand-in PipelineError via
        // the existing path. Falls back to a synthetic if needed.
        use colossus_extract::ExtractionSchema;
        let schema_err = ExtractionSchema::from_file(std::path::Path::new(
            "/nonexistent/path/should/never/exist.json",
        ))
        .expect_err("missing schema file should fail to load");
        let err = LlmExtractError::SchemaLoadFailed {
            schema_file: "missing.json".into(),
            source: schema_err,
        };
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(
            msg.contains("missing.json"),
            "msg must name the schema: {msg}"
        );
    }

    #[test]
    fn classify_response_not_json_is_terminal() {
        // ResponseNotJson carries an inner serde_json::Error. We
        // generate one via a parse failure.
        let serde_err = serde_json::from_str::<serde_json::Value>("not-json-text")
            .expect_err("malformed JSON must error");
        let err = LlmExtractError::ResponseNotJson {
            preview: "garbage llm output".into(),
            source: serde_err,
        };
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(msg.contains("non-JSON"), "msg must say what's wrong: {msg}");
        assert!(
            msg.contains("garbage llm output"),
            "msg must include preview: {msg}"
        );
    }

    #[test]
    fn classify_entity_serialization_failed_is_terminal() {
        let serde_err = serde_json::from_str::<serde_json::Value>("not-json-text")
            .expect_err("malformed JSON must error");
        let err = LlmExtractError::EntitySerializationFailed {
            entity_index: 7,
            source: serde_err,
        };
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(
            msg.contains("programming bug"),
            "msg must call out the bug class: {msg}"
        );
    }

    #[test]
    fn classify_relationship_serialization_failed_is_terminal() {
        let serde_err = serde_json::from_str::<serde_json::Value>("not-json-text")
            .expect_err("malformed JSON must error");
        let err = LlmExtractError::RelationshipSerializationFailed {
            rel_index: 3,
            source: serde_err,
        };
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass2", &err);
        assert!(is_terminal(&c));
    }

    #[test]
    fn classify_prompt_build_failed_is_terminal() {
        // PromptBuildFailed carries a colossus_extract::PipelineError. We
        // synthesize one through the same source error path the schema
        // test uses.
        use colossus_extract::ExtractionSchema;
        let pe =
            ExtractionSchema::from_file(std::path::Path::new("/nonexistent/prompt/schema.json"))
                .expect_err("missing schema should fail");
        let err = LlmExtractError::PromptBuildFailed { source: pe };
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(
            msg.contains("template"),
            "msg must point at template: {msg}"
        );
    }

    // ── Retryable variants ──────────────────────────────────────

    #[test]
    fn classify_llm_call_failed_is_retryable() {
        use colossus_extract::ExtractionSchema;
        let pe = ExtractionSchema::from_file(std::path::Path::new("/nonexistent.json"))
            .expect_err("missing should fail");
        let err = LlmExtractError::LlmCallFailed { source: pe };
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
        assert!(!is_terminal(&c), "LlmCallFailed must be retryable: {c:?}");
        let msg = display_message(&c);
        assert!(msg.contains("Will retry"), "msg must signal retry: {msg}");
    }

    #[test]
    fn classify_semaphore_closed_is_retryable() {
        let err = LlmExtractError::SemaphoreClosed;
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
        assert!(!is_terminal(&c), "SemaphoreClosed must be retryable");
    }

    #[test]
    fn classify_insert_run_failed_is_retryable() {
        let err = LlmExtractError::InsertRunFailed {
            message: "connection refused".into(),
        };
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
        assert!(!is_terminal(&c));
    }

    #[test]
    fn classify_complete_run_failed_is_retryable() {
        let err = LlmExtractError::CompleteRunFailed {
            message: "tx timeout".into(),
        };
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
        assert!(!is_terminal(&c));
    }

    #[test]
    fn classify_store_failed_is_retryable() {
        let err = LlmExtractError::StoreFailed {
            message: "deadlock detected".into(),
        };
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
        assert!(!is_terminal(&c));
    }

    // ── Unknown error type (downcast miss) ──────────────────────

    #[test]
    fn classify_dyn_unknown_error_is_retryable() {
        // A non-LlmExtractError boxed error — e.g. a sqlx::Error
        // promoted to Box<dyn Error>. The downcast misses and we
        // fall back to retryable to avoid locking up on a transient
        // we couldn't classify.
        let boxed: Box<dyn Error + Send + Sync> = "sudden infrastructure blip".into();
        let c = classify_dyn_llm_error("doc-x", "llm_extract_pass1", boxed);
        assert!(
            !is_terminal(&c),
            "unknown error must default to retryable: {c:?}"
        );
        let msg = display_message(&c);
        assert!(
            msg.contains("unclassified"),
            "msg must signal unknown type: {msg}"
        );
    }
}
