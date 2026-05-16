//! `DocumentPipeline` workflow — Phase 2-2c (Part 2): all 8 steps real.
//!
//! ## Purpose
//!
//! Phase 2's document-processing workflow. Replaces P1's echo step
//! with the 8-step pipeline. As of P2-2c Part 2, all 8 steps have
//! real implementations — zero placeholders remain.
//!
//! The 8 steps, in order:
//!
//! 1. `extract_text` — PDF/DOCX/TXT → `document_text` rows (REAL).
//! 2. `llm_extract_pass1` — LLM extraction + chunking (REAL).
//! 3. `llm_extract_pass2` — relationship extraction (REAL; skipped
//!    by the handler when the profile has `run_pass2 = false`).
//! 4. `verify` — grounding verification (REAL).
//! 5. `auto_approve` — auto-approve grounded items (REAL).
//! 6. `ingest` — write to Neo4j (REAL; cleanup-then-write idempotency).
//! 7. `index` — embed and write to Qdrant (REAL; native upsert idempotency).
//! 8. `completeness` — completeness check (REAL; terminal step).
//!
//! Each step is its own `ctx.run()` call so Restate journals each
//! step's outcome separately and replay can resume from the last
//! completed step.
//!
//! ## Why chunking is NOT a separate step
//!
//! In the prior draft of this design chunking lived between extract
//! and pass-1. The schema makes that awkward — `extraction_chunks`
//! rows carry an `extraction_run_id` FK and are pass-scoped (pass 1
//! chunks vs pass 2 chunks live under different `extraction_runs`
//! rows). Splitting "chunk" into its own pre-extraction step would
//! force one of: (a) create a placeholder run row up-front, (b)
//! introduce a pass-agnostic chunks table, or (c) recompute chunks
//! inside pass-1 anyway. All three are worse than just keeping the
//! chunk split where it already is — inside `llm_extract_pass1` /
//! `llm_extract_pass2`. The workflow is 8 steps, not 9.
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

use std::sync::Arc;

use restate_sdk::prelude::*;

use crate::models::document_status::{
    STATUS_APPROVED, STATUS_INDEXED, STATUS_INGESTED, STATUS_TEXT_EXTRACTED, STATUS_VERIFIED,
};
use crate::pipeline::context::AppContext;
use crate::pipeline::workflow_steps;

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

// ── Per-step status sentinels ────────────────────────────────────
//
// One string per terminal-per-step status the workflow writes to
// `STATUS_STATE_KEY` after each `ctx.run()` completes. The values
// that also map onto a `documents.status` column write are imported
// from `crate::models::document_status` so the casing
// (`TEXT_EXTRACTED`, `VERIFIED`, …) matches the canonical SQL
// vocabulary that `compute_status_group` and the legacy pipeline
// already enforce. The two intermediate states that do NOT have a
// `documents.status` counterpart (between-passes, pre-verify) live
// here as local constants. Naming each transition through a constant
// prevents typo drift and keeps the status vocabulary auditable.
//
// CONST justification (shared with the canonical module): these
// strings are pattern-matched by external callers (Restate admin
// UI, future `get_status` extensions, frontend `compute_status_group`).
// They are not env-var configurable.

const STATUS_PASS1_COMPLETE: &str = "PASS1_COMPLETE";
const STATUS_PASS2_COMPLETE: &str = "PASS2_COMPLETE";

/// Operator recovery hint included in every step-failure log line.
///
/// The architecture-review's "WHAT-TO-DO" requirement: a step-failure
/// log alone is not actionable without telling the operator where to
/// look. Restate journals the failure with the original `HandlerError`
/// message (which `classify_extract_error` populates with step-specific
/// guidance for the extract step). The same hint applies to every step,
/// so it lives compiled-in and is attached as a `recovery` structured
/// field on each `tracing::error!`.
///
/// CONST: free-form operator guidance — not env-var configurable. The
/// referenced URL is intentionally an env-var name (`RESTATE_ADMIN_URL`)
/// rather than a literal URL, so a deployment can point operators at
/// the right console without touching this file.
const STEP_FAILURE_RECOVERY: &str =
    "Inspect the Restate workflow journal for this doc_id (admin UI at \
     $RESTATE_ADMIN_URL) — terminal errors need fix+redeploy; retryable \
     errors auto-retry with exponential backoff.";

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
/// Holds an `Arc<AppContext>` so step handlers can reach the database
/// pools, the LLM/embedding providers, the HTTP client, and the
/// pipeline-configuration registry. Constructed once at process
/// startup in `restate_endpoint::serve_restate_endpoint` and shared
/// (by `Arc` clone) into every `ctx.run()` closure that needs it.
///
/// ## Rust Learning: why `Arc` here and `&self` on the handler
///
/// Restate's macro-generated `Service::serve` takes `self` by value,
/// but each handler call sees `&self`. Cloning the `Arc<AppContext>`
/// inside the handler body (one atomic increment) is what lets us move
/// the context into a `'static + Send` `async move` closure for
/// `ctx.run` without forcing the whole context to be `Clone`.
pub struct DocumentPipelineImpl {
    /// Shared application context. Cloned (cheap, refcount bump) into
    /// every `ctx.run` closure that needs access to the database
    /// pools, providers, or registry.
    pub ctx: Arc<AppContext>,
}

impl DocumentPipelineImpl {
    /// Construct a new workflow impl from a shared [`AppContext`].
    ///
    /// Called once from `restate_endpoint::serve_restate_endpoint` —
    /// the Restate SDK holds the resulting value for the lifetime of
    /// the HTTP/2 server.
    pub fn new(ctx: Arc<AppContext>) -> Self {
        Self { ctx }
    }
}

impl DocumentPipeline for DocumentPipelineImpl {
    async fn run(&self, ctx: WorkflowContext<'_>, doc_id: String) -> Result<String, HandlerError> {
        tracing::info!(doc_id = %doc_id, "DocumentPipeline workflow started");

        // ── Step 1: extract_text (REAL) ────────────────────────────
        //
        // ## Rust Learning: `Arc` clone before the async move
        //
        // Each `ctx.run` closure becomes a 'static future, so it must
        // own its captures. Cloning the `Arc<AppContext>` is a single
        // atomic refcount bump — the underlying `AppContext` is shared
        // across all clones. The doc_id is `.clone()`d separately
        // because the trailing tracing line and the next step both
        // need their own copies.
        let app = Arc::clone(&self.ctx);
        let did = doc_id.clone();
        ctx.run(
            || async move { workflow_steps::extract_text::step_extract_text(&app, &did).await },
        )
        .await
        .map_err(|e| {
            tracing::error!(
                doc_id = %doc_id, step = "extract_text", error = %e,
                recovery = STEP_FAILURE_RECOVERY,
                "DocumentPipeline step failed"
            );
            e
        })?;
        ctx.set(STATUS_STATE_KEY, STATUS_TEXT_EXTRACTED.to_string());

        // ── Step 2: llm_extract_pass1 (REAL — includes chunking) ──
        //
        // Chunking lives inside this step rather than as its own step
        // because `extraction_chunks` rows carry an `extraction_run_id`
        // FK and are pass-scoped (see the module-level doc above for
        // why a separate chunk step is awkward).
        let app = Arc::clone(&self.ctx);
        let did = doc_id.clone();
        ctx.run(
            || async move { workflow_steps::llm_extract::step_llm_extract_pass1(&app, &did).await },
        )
        .await
        .map_err(|e| {
            tracing::error!(
                doc_id = %doc_id, step = "llm_extract_pass1", error = %e,
                recovery = STEP_FAILURE_RECOVERY,
                "DocumentPipeline step failed"
            );
            e
        })?;
        ctx.set(STATUS_STATE_KEY, STATUS_PASS1_COMPLETE.to_string());

        // ── Step 3: llm_extract_pass2 (REAL — relationships) ───────
        //
        // The workflow body calls pass-2 unconditionally; the step
        // handler itself short-circuits when the resolved profile has
        // `run_pass2 = false`. No FSM routing here (the legacy
        // worker's `next_step_after_pass1` is bypassed on the Restate
        // path).
        let app = Arc::clone(&self.ctx);
        let did = doc_id.clone();
        ctx.run(
            || async move { workflow_steps::llm_extract::step_llm_extract_pass2(&app, &did).await },
        )
        .await
        .map_err(|e| {
            tracing::error!(
                doc_id = %doc_id, step = "llm_extract_pass2", error = %e,
                recovery = STEP_FAILURE_RECOVERY,
                "DocumentPipeline step failed"
            );
            e
        })?;
        ctx.set(STATUS_STATE_KEY, STATUS_PASS2_COMPLETE.to_string());

        // ── Step 4: verify (REAL) ──────────────────────────────────
        let app = Arc::clone(&self.ctx);
        let did = doc_id.clone();
        ctx.run(|| async move { workflow_steps::verify::step_verify(&app, &did).await })
            .await
            .map_err(|e| {
                tracing::error!(
                    doc_id = %doc_id, step = "verify", error = %e,
                    recovery = STEP_FAILURE_RECOVERY,
                    "DocumentPipeline step failed"
                );
                e
            })?;
        ctx.set(STATUS_STATE_KEY, STATUS_VERIFIED.to_string());

        // ── Step 5: auto_approve (REAL) ────────────────────────────
        //
        // Per P2-2c design decision (option b), the handler does NOT
        // write `documents.status` — the lifecycle column stays at
        // "VERIFIED" until step_ingest writes "INGESTED". The
        // Restate state still transitions through STATUS_APPROVED
        // below so the journal records the step boundary.
        let app = Arc::clone(&self.ctx);
        let did = doc_id.clone();
        ctx.run(
            || async move { workflow_steps::auto_approve::step_auto_approve(&app, &did).await },
        )
        .await
        .map_err(|e| {
            tracing::error!(
                doc_id = %doc_id, step = "auto_approve", error = %e,
                recovery = STEP_FAILURE_RECOVERY,
                "DocumentPipeline step failed"
            );
            e
        })?;
        ctx.set(STATUS_STATE_KEY, STATUS_APPROVED.to_string());

        // ── Step 6: ingest (REAL) ──────────────────────────────────
        //
        // The Postgres `documents.status = "INGESTED"` write happens
        // inside the core `run_ingest`. The core also performs
        // cleanup-then-write idempotency (calls `cleanup_neo4j` at
        // the start of every invocation), so Restate replay is safe
        // even though `ingest_helpers` uses CREATE rather than MERGE.
        let app = Arc::clone(&self.ctx);
        let did = doc_id.clone();
        ctx.run(|| async move { workflow_steps::ingest::step_ingest(&app, &did).await })
            .await
            .map_err(|e| {
                tracing::error!(
                    doc_id = %doc_id, step = "ingest", error = %e,
                    recovery = STEP_FAILURE_RECOVERY,
                    "DocumentPipeline step failed"
                );
                e
            })?;
        ctx.set(STATUS_STATE_KEY, STATUS_INGESTED.to_string());

        // ── Step 7: index (REAL) ───────────────────────────────────
        //
        // The Postgres `documents.status = "INDEXED"` write happens
        // inside the core `run_index`. Qdrant upsert is natively
        // idempotent — Restate replay produces identical points.
        let app = Arc::clone(&self.ctx);
        let did = doc_id.clone();
        ctx.run(|| async move { workflow_steps::index::step_index(&app, &did).await })
            .await
            .map_err(|e| {
                tracing::error!(
                    doc_id = %doc_id, step = "index", error = %e,
                    recovery = STEP_FAILURE_RECOVERY,
                    "DocumentPipeline step failed"
                );
                e
            })?;
        ctx.set(STATUS_STATE_KEY, STATUS_INDEXED.to_string());

        // ── Step 8: completeness (REAL — terminal step) ────────────
        //
        // The Postgres `documents.status = "PUBLISHED"` write happens
        // inside the core `run_completeness`, so no handler-level
        // status write is needed. The Restate state still transitions
        // through STATUS_COMPLETED at the very end (below) to mark
        // the workflow journal as terminally complete.
        let app = Arc::clone(&self.ctx);
        let did = doc_id.clone();
        ctx.run(
            || async move { workflow_steps::completeness::step_completeness(&app, &did).await },
        )
        .await
        .map_err(|e| {
            tracing::error!(
                doc_id = %doc_id, step = "completeness", error = %e,
                recovery = STEP_FAILURE_RECOVERY,
                "DocumentPipeline step failed"
            );
            e
        })?;

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
