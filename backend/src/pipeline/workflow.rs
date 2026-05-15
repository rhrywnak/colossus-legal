//! `DocumentPipeline` workflow — Phase 1 echo validator.
//!
//! ## Purpose
//!
//! This is the **Phase 1 milestone**: a minimal Restate workflow that
//! proves the SDK integration is wired end-to-end. It is deliberately
//! trivial — one `ctx.run()` step that echoes the document id, one
//! `ctx.set()` that records "completed", one shared handler that
//! reads it back.
//!
//! Phase 2 replaces the echo step with the real document-processing
//! pipeline (ingest → extract → verify → index, currently in
//! `pipeline::steps::*`). For now the goal is to validate four things
//! once the binary is deployed against the DEV Restate server:
//!
//! 1. The `#[restate_sdk::workflow]` macro compiles in our backend.
//! 2. The workflow registers with the Restate discovery endpoint.
//! 3. A workflow invocation reaches the SDK and runs to completion.
//! 4. `ctx.run()` journaling and `ctx.set()` / `ctx.get()` state both
//!    work against the live Restate journal.
//!
//! ## What gets journaled (replay semantics)
//!
//! `ctx.run(|| async { ... })` journals the **return value** of the
//! closure, not the closure itself. On first execution Restate runs
//! the closure once, captures the returned `T`, writes it to the
//! journal. On retries (or replay after process crash), the SDK reads
//! the journaled value back and **skips the closure entirely** — so
//! the closure's side effects can fire at most once per workflow
//! invocation. That is the durability guarantee.
//!
//! ## What MUST NOT happen inside `ctx.run()`
//!
//! Per the SDK doc (`restate-sdk-0.6/src/context/mod.rs:685`):
//! > You cannot use the Restate context within `ctx.run`. This
//! > includes actions such as getting state, calling another service,
//! > and nesting other journaled actions.
//!
//! State writes (`ctx.set`), state reads (`ctx.get`), other
//! `ctx.run`s, and sleeps all live OUTSIDE the closure. Putting them
//! inside causes non-determinism errors during replay because the
//! journal entries get interleaved unpredictably.
//!
//! ## Replay-aware logging — known limitation
//!
//! `tracing::info!` calls inside the workflow body fire on first
//! execution AND on replay (per the SDK's
//! [`filter::ReplayAwareFilter`] doc). For Phase 1 we accept the
//! duplicate-log behaviour — the workflow is short and the operator
//! can dedupe by invocation id. P2+ should consider installing the
//! `ReplayAwareFilter` in the tracing-subscriber setup.

use restate_sdk::prelude::*;

// ── State contract ───────────────────────────────────────────────
//
// The `run` handler writes to one key (`status`); the `get_status`
// shared handler reads the same key. Naming both ends through
// constants prevents the typo-drift failure mode where one handler
// writes "status" while the other reads "Status" and silently returns
// "unknown".

/// State key under which `run` records the workflow's terminal status.
///
/// CONST: Restate journal state-contract key — not a config knob.
/// Changing this value at runtime would orphan the status entries of
/// every in-flight workflow instance (they were written under the old
/// key, polling callers would read the new key and see `"unknown"`
/// forever). Migration of this value requires coordinated draining
/// of in-flight workflows, not an env-var change.
const STATUS_STATE_KEY: &str = "status";

/// Value `run` writes once the echo step has been journaled.
///
/// CONST: state-contract sentinel that is part of the workflow's
/// external API surface. External consumers (Restate admin UI,
/// future P2 callers polling via `get_status`) pattern-match on this
/// exact string to detect terminal success. Renaming it would break
/// every poller in lockstep, so it lives compiled-in rather than
/// configurable.
const STATUS_COMPLETED: &str = "completed";

/// Sentinel `get_status` returns when the state key has never been
/// written — distinct from `"completed"` and from any future status
/// values so an external caller can distinguish "workflow has not
/// progressed past start" from "workflow finished successfully".
///
/// CONST: same justification as `STATUS_COMPLETED` — external
/// pattern-matchers depend on this exact string; it is not
/// runtime-configurable.
const STATUS_UNKNOWN: &str = "unknown";

/// Document-processing workflow.
///
/// In Phase 1 this is an echo validator (see module doc). In Phase 2
/// the `run` handler is rewritten to drive the actual document
/// pipeline; `get_status` and any future handlers stay as the
/// external-control surface.
///
/// ## Rust Learning: the `#[restate_sdk::workflow]` macro
///
/// The macro:
/// - Generates a `Service` impl on the user's struct via a hidden
///   `.serve()` accessor (so `DocumentPipelineImpl.serve()` is what
///   `Endpoint::builder().bind(...)` accepts).
/// - Injects the `&self` and `ctx: WorkflowContext<'_>` parameters
///   onto each handler when the trait is implemented — the trait
///   declaration here omits them deliberately.
/// - Marks `run` as the **exactly-once** handler. Any handler
///   annotated `#[shared]` becomes read-only and concurrently
///   invocable.
#[restate_sdk::workflow]
pub trait DocumentPipeline {
    /// Exactly-once orchestration handler.
    ///
    /// Receives the document id as a bare JSON-encoded string (e.g.
    /// `curl -d '"test-doc-id"'`). Returns the terminal status as a
    /// string. Future phases may evolve the input/output to dedicated
    /// structs.
    async fn run(doc_id: String) -> Result<String, HandlerError>;

    /// Read-only status accessor. Can be called any number of times,
    /// before, during, or after `run` completes.
    ///
    /// Returns `"completed"` once `run` has journaled the echo step,
    /// otherwise `"unknown"`.
    #[shared]
    async fn get_status() -> Result<String, HandlerError>;
}

/// Concrete implementation of [`DocumentPipeline`].
///
/// Unit struct because P1-7 carries no per-workflow state outside of
/// what Restate's own journal holds. P2 will likely promote this to a
/// struct holding `Arc<AppContext>` so the real pipeline steps can
/// reach the LLM/embedding engines and the database pools.
pub struct DocumentPipelineImpl;

impl DocumentPipeline for DocumentPipelineImpl {
    async fn run(&self, ctx: WorkflowContext<'_>, doc_id: String) -> Result<String, HandlerError> {
        tracing::info!(doc_id = %doc_id, "DocumentPipeline workflow started");

        // ## Rust Learning: `async move` inside a `ctx.run` closure
        //
        // The closure outlives the synchronous call frame — Restate
        // stashes it on the journal task. So it must own everything
        // it touches. `doc_id` is still needed by the trailing log
        // line after `.await`, so we clone it for the closure and
        // keep the original. `async move` captures the clone
        // by-value.
        //
        // The `Ok::<String, HandlerError>(...)` turbofish is needed
        // because the closure body has only one statement — Rust
        // can't infer which `Result<_, _>` variant the `Ok` belongs
        // to without an explicit type.
        let echo_doc_id = doc_id.clone();
        let echo_result: String = ctx
            .run(|| async move { Ok::<String, HandlerError>(format!("echo: {echo_doc_id}")) })
            .await
            .map_err(|e| {
                // `ctx.run` propagates `HandlerError` opaquely to the
                // Restate runtime. Without this `map_err` the operator
                // would see a generic SDK error in the Restate journal
                // with no indication of which document failed. Logging
                // here records doc_id + step name in our own tracing
                // pipeline before forwarding the error unchanged.
                tracing::error!(
                    doc_id = %doc_id,
                    step = "echo",
                    error = %e,
                    "DocumentPipeline echo ctx.run step failed"
                );
                e
            })?;

        // Useful at trace level when debugging replay behaviour
        // against a live Restate server: on first execution this logs
        // the freshly-computed value; on replay it logs the journaled
        // value (which should be identical — divergence indicates a
        // non-deterministic step bug).
        tracing::debug!(
            doc_id = %doc_id,
            echo = %echo_result,
            "Echo step journaled"
        );

        // ctx.set is synchronous (no .await) and fire-and-forget at
        // the API level — Restate journals the write transactionally
        // with the surrounding handler completion. `&str` does not
        // impl the SDK's `Serialize` trait (only owned `String`
        // does), hence the `.to_string()`.
        ctx.set(STATUS_STATE_KEY, STATUS_COMPLETED.to_string());

        tracing::info!(doc_id = %doc_id, "DocumentPipeline workflow completed");
        Ok(STATUS_COMPLETED.to_string())
    }

    async fn get_status(&self, ctx: SharedWorkflowContext<'_>) -> Result<String, HandlerError> {
        // The workflow instance key is the doc_id at the Restate
        // layer — surfaced here so operators tailing logs can
        // correlate a status read with the document being queried.
        // `debug` (not `info`) because this is a polling read
        // endpoint; an `info` here would flood logs during normal
        // operation.
        let workflow_key = ctx.key().to_string();
        tracing::debug!(
            workflow_key = %workflow_key,
            "DocumentPipeline get_status invoked"
        );

        // `ctx.get` returns `Result<Option<T>, TerminalError>`:
        // - `Ok(None)` means the key has never been written for this
        //   workflow instance (run hasn't reached the set() call, or
        //   was never invoked).
        // - `Err(_)` means the underlying state read failed at the
        //   Restate journal layer — we wrap with a tracing::error!
        //   carrying the workflow key so the failure is observable
        //   without parsing the Restate journal, then forward the
        //   error unchanged.
        let status: Option<String> = ctx.get(STATUS_STATE_KEY).await.map_err(|e| {
            tracing::error!(
                workflow_key = %workflow_key,
                state_key = STATUS_STATE_KEY,
                error = %e,
                "DocumentPipeline get_status: state read failed"
            );
            e
        })?;
        Ok(status.unwrap_or_else(|| STATUS_UNKNOWN.to_string()))
    }
}
