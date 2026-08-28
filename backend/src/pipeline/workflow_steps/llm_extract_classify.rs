//! Terminal-vs-retryable classification for the LLM extraction steps.
//!
//! ## Why this is its own module
//!
//! Split out of `llm_extract.rs` on 2026-08-28, when the LLM-call arm stopped
//! being a fixed verdict and became a function of the retry policy. That made
//! this the place where a money-spending decision is taken, which is worth a
//! module doc of its own — and it brought the step-handler file back under the
//! 300-line budget it had drifted past.
//!
//! ## What the classification actually decides
//!
//! Restate re-invokes a handler that returns a retryable `HandlerError`, and
//! stops at a `TerminalError`. For every failure that costs money to reproduce,
//! that is a spending decision, so the rules are:
//!
//! - **Configuration and state** → terminal. The retry sees the same state.
//! - **LLM output bugs** (non-JSON, serialization) → terminal. Template or
//!   prompt drift needs a human.
//! - **Truncation** → terminal at ANY retry cap. The ceiling that cut the
//!   response off is still the ceiling on the retry (census R-3, 2026-08-27).
//! - **The LLM call itself** → decided by `LLM_RETRY_MAX`, which is zero by
//!   default (2026-08-28). See [`crate::llm_retry`].
//! - **Database and semaphore** → retryable. These cost nothing to retry.

use std::error::Error;

use restate_sdk::errors::{HandlerError, TerminalError};

use crate::pipeline::steps::llm_extract::LlmExtractError;

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
pub(crate) fn classify_dyn_llm_error(
    doc_id: &str,
    step_name: &'static str,
    e: Box<dyn Error + Send + Sync>,
    retry_max: u32,
) -> HandlerError {
    match e.downcast::<LlmExtractError>() {
        Ok(typed) => classify_llm_extract_error(doc_id, step_name, &typed, retry_max),
        Err(boxed) => HandlerError::from(format!(
            "step_{step_name}: unclassified failure for '{doc_id}': {boxed}. \
             Will retry."
        )),
    }
}

/// The money decision: may the engine re-invoke a failed LLM call by itself?
///
/// `LLM_RETRY_MAX == 0` (the shipped default) makes the failure TERMINAL, so the
/// run stops and waits for a human to click Re-process. Any higher value hands
/// the decision back to Restate's retry loop. Both messages name the key, so an
/// operator reading `pipeline_jobs.error` learns which policy was in force
/// without having to go and look it up.
///
/// See [`crate::llm_retry`] for why the default is zero.
fn classify_llm_call_failure(
    doc_id: &str,
    step_name: &'static str,
    e: &LlmExtractError,
    retry_max: u32,
) -> HandlerError {
    if retry_max == 0 {
        TerminalError::new(format!(
            "step_{step_name}: LLM call failed for '{doc_id}'. {e}. No automatic retry — \
             LLM_RETRY_MAX is 0, so this run is FAILED and waits for a human to click \
             Re-process. Raise LLM_RETRY_MAX if automatic retries are wanted."
        ))
        .into()
    } else {
        HandlerError::from(format!(
            "step_{step_name}: LLM call failed for '{doc_id}'. {e}. LLM_RETRY_MAX is \
             {retry_max}, so this will retry."
        ))
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
pub(crate) fn classify_llm_extract_error(
    doc_id: &str,
    step_name: &'static str,
    e: &LlmExtractError,
    retry_max: u32,
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

        // ── Terminal: the response was cut off at the ceiling ────
        //
        // A truncation is DETERMINISTIC, which is what makes it terminal
        // rather than retryable. The retry would send the same prompt to the
        // same model under the same `max_tokens` and be cut off in the same
        // place; Restate's backoff would spend real money and wall-clock
        // arriving at the identical failure. Only a human raising the cap in
        // the profile YAML changes the outcome.
        //
        // The gate's own message already names the model, the cap, what was
        // produced against it, and the remedy, so it is passed through whole —
        // `{e}` here is `ResponseTruncated`'s Display, which wraps it. What is
        // deliberately absent is the "Will retry." sentence the retryable arm
        // below ends with: this step will not retry, and telling an operator
        // otherwise while the run sits FAILED is the failure mode Standing
        // Rule 1 exists to prevent.
        // The doc id leads rather than trails: the gate's message is a
        // multi-sentence paragraph ending in its own remedy ("…and re-run."),
        // so appending "for 'doc-x'" after it would read as a fragment.
        E::ResponseTruncated { .. } => TerminalError::new(format!(
            "step_{step_name}: document '{doc_id}': {e} No retry — a truncated \
             response is deterministic, and re-running against the same cap \
             produces the same truncation."
        ))
        .into(),

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

        // ── The LLM call itself: classified by the RETRY POLICY ──
        //
        // This arm used to be folded in with the database failures below, as
        // unconditionally retryable. On 2026-08-28 a 36-page transcript failed
        // at exactly 600.0s — the whole-request timeout on a non-streaming call,
        // fired while Opus 5 was healthily generating — and Restate dutifully
        // re-ran the step and spent the same money reaching the same wall. That
        // is the second deterministic failure this month to be paid for twice
        // (the first was truncation; see the arm above).
        //
        // So the engine's re-run decision now reads the SAME number the retry
        // loop in `crate::llm_retry` reads. At the shipped `LLM_RETRY_MAX=0` an
        // LLM-call failure is TERMINAL: the run stops, and a human decides
        // whether it is worth another call by clicking Re-process. Raising the
        // key hands the decision back to the engine — one config change
        // governing both halves, which is the only way they cannot disagree.
        //
        // Note this is deliberately NOT a judgement about which failures are
        // transient. Some of them genuinely are. The ruling is that spending
        // money on that guess is a human's call, not the runtime's.
        E::LlmCallFailed { .. } => classify_llm_call_failure(doc_id, step_name, e, retry_max),

        // ── Retryable: transient infrastructure ──────────────────
        //
        // Database and semaphore failures, which cost nothing to retry and
        // routinely resolve on their own. The retry cap above governs PAID
        // calls; it deliberately does not govern these.
        E::SemaphoreClosed
        | E::InsertRunFailed { .. }
        | E::CompleteRunFailed { .. }
        | E::StoreFailed { .. } => HandlerError::from(format!(
            "step_{step_name}: transient failure for '{doc_id}'. {e}. \
             Will retry."
        )),
    }
}

#[cfg(test)]
#[path = "llm_extract_classify_tests.rs"]
mod tests;
