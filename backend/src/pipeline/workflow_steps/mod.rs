//! Restate workflow step handlers.
//!
//! Each step is a thin async function called from within a `ctx.run()`
//! closure in the [`DocumentPipeline`](crate::pipeline::workflow)
//! workflow. Steps take `&Arc<AppContext>` and a document id, perform
//! their work against the database and external services, and return a
//! short summary string that Restate journals for replay.
//!
//! ## Why a sibling module to `steps/`
//!
//! `steps/` holds the legacy `colossus-pipeline`-driven steps that
//! implement `Step<DocProcessing>`. Those carry a `CancellationToken` /
//! `ProgressReporter` and a `Step::execute` shape that does not match
//! what Restate's `ctx.run` closures want. Rather than make every legacy
//! step also satisfy the Restate shape (or vice versa), the Restate
//! step handlers live here as plain `pub async fn` and delegate their
//! shared work to extracted helpers in `steps/` (e.g.
//! [`crate::pipeline::steps::extract_text::extract_text_to_db`]). This
//! keeps both paths thin: the legacy step wraps the helper with cancel
//! checks and progress reporting; the Restate handler wraps the same
//! helper with the journal-summary formatting and Restate's
//! `HandlerError` / `TerminalError` error classification.
//!
//! ## Idempotency
//!
//! Every step in this module checks whether its work has already been
//! done before proceeding. Restate replays the `ctx.run` closure on
//! workflow recovery, so each closure body must be safe to run more
//! than once against the same database state — the idempotency check
//! is what makes that safe. The DB layer is friendly to this pattern
//! (`document_text` upserts on `(document_id, page_number)`), but the
//! step body still short-circuits when prior work is observable, both
//! to save the work itself and to keep the journal entry small.
//!
//! ## Execution-history recording
//!
//! Every step handler writes a row to `pipeline_steps` so the
//! Execution History panel and the legacy `/history` API surface the
//! same per-step audit trail for both the Worker and the Restate
//! paths. The three helpers below — [`begin_step`],
//! [`finish_step_success`], [`finish_step_failure`] — wrap the
//! [`crate::repositories::pipeline_repository::steps`] functions with
//! the Restate-flavor error semantics:
//!
//! - `begin_step` failures are RETRYABLE — a transient DB blip on the
//!   first call must not kill the workflow. Restate retries the
//!   closure body until the row insert succeeds.
//! - `finish_step_success` and `finish_step_failure` are BEST-EFFORT
//!   — they log a `tracing::warn!` on DB error but do not propagate.
//!   The step's actual success/failure outcome (which the workflow
//!   already knows) takes priority over the audit row.
//!
//! Restate journals the closure return value, so on workflow replay
//! the closure body — including these helpers — is skipped entirely
//! and no duplicate rows are written. On a retry (after a retryable
//! step error) the closure re-executes fully and a fresh row is
//! inserted per attempt, mirroring the legacy Worker's per-attempt
//! audit semantics.

use std::future::Future;
use std::time::Instant;

use restate_sdk::errors::HandlerError;
use sqlx::PgPool;

use crate::repositories::pipeline_repository::steps;

pub mod auto_approve;
pub mod completeness;
pub mod extract_text;
pub mod index;
pub mod ingest;
pub mod llm_extract;
pub mod verify;

// ── Step name constants ─────────────────────────────────────────
//
// CONST justification: each value below is the canonical step name
// written into `pipeline_steps.step_name`, read by the frontend's
// Execution History panel and the registry's `step_label()` lookup
// (which keys into the `step_labels:` YAML section by exact string
// match). They are not env-var configurable: changing one of these
// values at runtime would orphan every `pipeline_steps` row already
// in the DB and break the registry lookup in `step_progress.rs`.
//
// The strings here match the keys in `backend/config/pipeline_registry.yaml`'s
// `step_labels:` section and the match arms in
// `PipelineRegistry::step_label()` in `pipeline/registry.rs`. Renaming
// any of these constants requires a coordinated edit of all three.

/// Step name for the text-extraction step (PDF/DOCX/TXT → `document_text`).
pub const STEP_EXTRACT_TEXT: &str = "extract_text";

/// Step name for the LLM extraction pass-1 step (chunking + entity extraction).
pub const STEP_LLM_EXTRACT_PASS1: &str = "llm_extract_pass1";

/// Step name for the LLM extraction pass-2 step (relationship extraction).
pub const STEP_LLM_EXTRACT_PASS2: &str = "llm_extract_pass2";

/// Step name for the canonical-text grounding-verification step.
pub const STEP_VERIFY: &str = "verify";

/// Step name for the bulk-approve step (grounded items → APPROVED).
pub const STEP_AUTO_APPROVE: &str = "auto_approve";

/// Step name for the Neo4j-ingest step.
pub const STEP_INGEST: &str = "ingest";

/// Step name for the Qdrant-index step (embed + upsert).
pub const STEP_INDEX: &str = "index";

/// Step name for the terminal completeness-check step.
pub const STEP_COMPLETENESS: &str = "completeness";

// ── Execution-history recording helpers ─────────────────────────

/// Sentinel written to `pipeline_steps.triggered_by` whenever the
/// Restate workflow runs a step.
///
/// CONST: distinguishes Restate-path rows from legacy Worker rows
/// (which write the operator username) and from the
/// `colossus-pipeline` framework's `PgStepRecorder` (which writes
/// `"worker"`). Not env-var configurable — it's the audit-trail
/// label for THIS code path.
const TRIGGERED_BY_RESTATE: &str = "restate";

/// Outcome returned by each step handler's body function.
///
/// The body computes the journal summary string AND the JSON written
/// into `pipeline_steps.result_summary`, returning both via this
/// struct so the wrapping `step_X` function can hand them off to the
/// recording helpers without re-deriving anything.
///
/// ## Rust Learning: a struct return vs a tuple
///
/// Using a named struct here (instead of `Result<(String, Value, bool)>`)
/// is purely for readability — three fields with the same `String`/`bool`
/// types as a tuple would be easy to swap the order of accidentally,
/// and a future fourth field can be added by name without touching
/// every call site.
pub struct StepOutcome {
    /// Short summary string returned from the handler. Restate
    /// journals this on the success path.
    pub summary: String,

    /// JSON written into `pipeline_steps.result_summary`. Each step
    /// has its own shape, replicating what the legacy worker's
    /// `progress.set_step_result(...)` call emitted so external
    /// audit tooling sees the same column content for both paths.
    pub result_summary: serde_json::Value,

    /// True when the body short-circuited before doing real work
    /// (e.g., the extract-text idempotency guard found existing
    /// pages). [`finish_step_success`] records `duration_secs = 0.0`
    /// in this case — no wall-clock work was done, so the recorded
    /// duration would otherwise reflect only the overhead of the
    /// short-circuit checks.
    pub skipped_early: bool,
}

/// Insert a `pipeline_steps` row in `'running'` state and return the
/// row id.
///
/// Maps `sqlx::Error` to a RETRYABLE `HandlerError`: a transient DB
/// blip on this first call must not kill the workflow. Restate's
/// exponential backoff will re-run the entire `ctx.run` closure body
/// (including this `begin_step` call) on the next attempt; eventually
/// either the DB recovers and the row inserts, or Restate's retry
/// budget is exhausted and the workflow fails over to the
/// documents-table failure write in `workflow.rs`.
///
/// ## Why retryable, not terminal
///
/// The original Step-1 instruction proposed making this terminal
/// ("we can't track the step"). Roman's Step-2 decision: a missing
/// audit row is a non-fatal observability gap; a refused DB
/// connection is a transient infrastructure issue. Letting Restate
/// retry preserves both signals — if the DB stays broken, the
/// workflow fails AFTER the retry budget runs out; if it recovers,
/// the audit row gets written on the next attempt.
///
/// ## Rust Learning: `&'static str` parameter
///
/// The `step_name` parameter is `&'static str` because every call
/// site passes one of the `STEP_*` const values above. The borrow is
/// `'static` so it can be plumbed into `#[instrument]`'s field
/// without lifetime contortions and reused inside the error message
/// without cloning.
#[tracing::instrument(skip(db), fields(doc_id = %doc_id, step = step_name))]
pub async fn begin_step(
    db: &PgPool,
    doc_id: &str,
    step_name: &'static str,
) -> Result<i32, HandlerError> {
    steps::record_step_start(
        db,
        doc_id,
        step_name,
        TRIGGERED_BY_RESTATE,
        &serde_json::json!({}),
    )
    .await
    .map_err(|e| {
        HandlerError::from(format!(
            "{step_name}: failed to record step start for '{doc_id}' \
             in pipeline_steps: {e}. Will retry."
        ))
    })
}

/// Best-effort: mark the `pipeline_steps` row as `'completed'`.
///
/// On DB error, logs a `tracing::warn!` and returns silently — the
/// step's actual success outcome is preserved. The audit-row write
/// is observability, not a critical path.
///
/// Calls [`compute_duration_secs`] to derive the value persisted to
/// `pipeline_steps.duration_secs`; see that helper's doc for the
/// `skipped_early` contract.
#[tracing::instrument(skip(db, result_summary), fields(step_id, skipped_early))]
pub async fn finish_step_success(
    db: &PgPool,
    step_id: i32,
    start: Instant,
    skipped_early: bool,
    result_summary: &serde_json::Value,
) {
    let duration_secs = compute_duration_secs(skipped_early, start);
    // best-effort: pipeline_steps recording is observability, not the
    // step's critical path. A DB write failure here leaves the row
    // stuck in 'running', which is visible in the Execution History
    // panel as a "hung" step — preferable to failing the actual step
    // for an audit-trail glitch.
    if let Err(e) = steps::record_step_complete(db, step_id, duration_secs, result_summary).await {
        tracing::warn!(
            step_id, duration_secs, error = %e,
            "record_step_complete failed (non-fatal — pipeline_steps row will stay 'running')"
        );
    }
}

/// Pure-logic helper: pick the `pipeline_steps.duration_secs` value
/// for a step that just completed successfully.
///
/// When `skipped_early` is `true`, returns `0.0` — the body
/// short-circuited (e.g., the extract-text idempotency guard fired
/// or pass-2 was profile-skipped) and did no meaningful wall-clock
/// work. Recording the actual elapsed time on a skip path would
/// reflect only the cheap short-circuit check and mislead an
/// operator who reads the Execution History panel's duration column
/// as "how long did this step's work take."
///
/// When `skipped_early` is `false`, returns `start.elapsed()` in
/// seconds — the standard wall-clock measurement.
///
/// Extracted from [`finish_step_success`] so the branch is
/// unit-testable without a database. The DB write itself is covered
/// by integration tests in `repositories::pipeline_repository::steps`.
fn compute_duration_secs(skipped_early: bool, start: Instant) -> f64 {
    if skipped_early {
        0.0
    } else {
        start.elapsed().as_secs_f64()
    }
}

/// Wrap a step's body with the full `pipeline_steps` lifecycle:
/// `record_step_start` → run the body → `record_step_{complete,failure}`.
///
/// Each `step_X` pub fn in this module is a one-call shim over this
/// helper. The shape lets the runtime details (acquiring the step
/// id, capturing the start instant, dispatching success vs failure)
/// live in one place rather than being copy-pasted into eight
/// near-identical match blocks.
///
/// ## Argument shape
///
/// `body` is a *future* (not a closure) by design. Each `step_X` pub
/// fn calls `step_X_body(app, doc_id)` — an `async fn` that returns
/// the future eagerly — and hands the future off without polling it.
/// The future captures the borrows of `app` and `doc_id` so the
/// helper does not need to thread those through its own signature.
/// Restate's `ctx.run` calls `step_X(...).await` from inside its
/// `'static + Send` closure; the borrows live for the entire
/// duration of that call, so the lifetimes work out without
/// additional bounds.
///
/// ## Rust Learning: `impl Future<Output = ...>` parameter
///
/// `impl Trait` in parameter position is "anonymous generic" —
/// equivalent to `<F: Future<...>>`. We use the impl-trait spelling
/// for two reasons: (1) the body type is unique per call site
/// (each `async fn` body has its own anonymous future type, so we
/// could not name them anyway), and (2) the call site reads more
/// naturally without the turbofish: `record_step_lifecycle(db, did,
/// STEP, step_X_body(app, did))` rather than
/// `record_step_lifecycle::<_>(...)`. The body future has captured
/// borrows internally so it is `!Send` in the general case — we
/// deliberately don't add a `Send` bound here so each handler's
/// body decides what borrow shape works for it.
#[tracing::instrument(skip(db, body), fields(doc_id = %doc_id, step = step_name))]
pub async fn record_step_lifecycle(
    db: &PgPool,
    doc_id: &str,
    step_name: &'static str,
    body: impl Future<Output = Result<StepOutcome, HandlerError>>,
) -> Result<String, HandlerError> {
    let step_id = begin_step(db, doc_id, step_name).await?;
    let start = Instant::now();
    match body.await {
        Ok(outcome) => {
            finish_step_success(
                db,
                step_id,
                start,
                outcome.skipped_early,
                &outcome.result_summary,
            )
            .await;
            Ok(outcome.summary)
        }
        Err(e) => {
            finish_step_failure(db, step_id, start, &e).await;
            Err(e)
        }
    }
}

/// Best-effort: mark the `pipeline_steps` row as `'failed'`.
///
/// Reads the `HandlerError`'s underlying `Display` (via `as_ref()`)
/// for the `error_message` column. That string includes any recovery
/// guidance the per-step `classify_*_error` function embedded, so the
/// Execution History panel's red error text gives the operator the
/// same actionable message they'd see in the Restate journal.
///
/// On DB error, logs a `tracing::warn!` and returns silently — same
/// rationale as [`finish_step_success`]. The step is already failing
/// for its own reason; an additional audit-write failure would only
/// add noise to the original signal.
#[tracing::instrument(skip(db, err), fields(step_id))]
pub async fn finish_step_failure(db: &PgPool, step_id: i32, start: Instant, err: &HandlerError) {
    let duration_secs = start.elapsed().as_secs_f64();
    let err_msg = handler_error_display(err);
    if let Err(e) = steps::record_step_failure(db, step_id, duration_secs, &err_msg).await {
        tracing::warn!(
            step_id, duration_secs, error = %e,
            "record_step_failure failed (non-fatal — pipeline_steps row will stay 'running')"
        );
    }
}

/// Pure-logic helper: extract the operator-facing `Display` string
/// from a `HandlerError` for persistence in
/// `pipeline_steps.error_message`.
///
/// ## Rust Learning: extracting the inner Display from a HandlerError
///
/// `HandlerError` itself does not implement `Display` (its inner
/// enum is `pub(crate)` to restate_sdk). It does implement
/// `AsRef<dyn StdError + Send + Sync>`, and every `StdError`
/// implements `Display`. Routing through `as_ref()` gives us the
/// full message — including the `"Terminal error [code]:"` /
/// `"Retryable error:"` prefix that classifies the failure for the
/// operator reading the row.
///
/// Extracted from [`finish_step_failure`] so the contract (terminal
/// errors prefixed `"Terminal error"`, retryable errors prefixed
/// `"Retryable error"`) can be pinned by a unit test — protecting
/// against a future `restate_sdk` version that silently changes the
/// inner `Display` impl.
fn handler_error_display(err: &HandlerError) -> String {
    let inner: &(dyn std::error::Error + Send + Sync) = err.as_ref();
    format!("{inner}")
}

// ── Unit tests (pure-logic helpers) ─────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use restate_sdk::errors::TerminalError;
    use std::time::Duration;

    #[test]
    fn compute_duration_secs_returns_zero_when_skipped_early() {
        // Even if some wall-clock time has elapsed since `start`, a
        // skipped-early step must report 0.0 — that's the audit-row
        // contract the Execution History panel reads.
        let start = Instant::now();
        std::thread::sleep(Duration::from_millis(5));
        let secs = compute_duration_secs(true, start);
        assert_eq!(
            secs, 0.0,
            "skipped_early=true must yield duration_secs=0.0, got {secs}"
        );
    }

    #[test]
    fn compute_duration_secs_returns_positive_elapsed_when_not_skipped() {
        let start = Instant::now();
        std::thread::sleep(Duration::from_millis(5));
        let secs = compute_duration_secs(false, start);
        assert!(
            secs > 0.0,
            "skipped_early=false must yield positive elapsed seconds, got {secs}"
        );
        // Sanity bound: 5ms sleep on a healthy CI is well under 1s.
        // If this trips, the test environment is under heavy load.
        assert!(
            secs < 1.0,
            "elapsed seconds for a 5ms sleep should be well under 1s, got {secs}"
        );
    }

    #[test]
    fn handler_error_display_extracts_terminal_prefix() {
        // Terminal errors must produce a message starting with
        // `"Terminal error"` so the Execution History panel's red
        // error text classifies the failure for the operator. This
        // pins the contract against a future restate_sdk upgrade
        // that might rename the inner Display prefix.
        let err: HandlerError =
            TerminalError::new("step_X: simulated failure with recovery hint".to_string()).into();
        let msg = handler_error_display(&err);
        assert!(
            msg.starts_with("Terminal error"),
            "terminal HandlerError must start with 'Terminal error', got: {msg}"
        );
        assert!(
            msg.contains("simulated failure"),
            "extracted message must include the original Display body, got: {msg}"
        );
    }

    #[test]
    fn handler_error_display_extracts_retryable_prefix() {
        let err: HandlerError =
            HandlerError::from("step_X: transient DB blip for 'doc-x'. Will retry.".to_string());
        let msg = handler_error_display(&err);
        assert!(
            msg.starts_with("Retryable error"),
            "non-terminal HandlerError must start with 'Retryable error', got: {msg}"
        );
        assert!(
            msg.contains("Will retry"),
            "extracted message must include the original Display body, got: {msg}"
        );
    }
}
