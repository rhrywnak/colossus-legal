//! Per-step UI-progress writes driven by the registry's `step_labels`
//! configuration.
//!
//! Every step's core function calls [`write_start`] when it begins
//! and [`write_end`] when it completes. Both functions read the
//! `label` string and `percent_start` / `percent_end` workflow-level
//! percentages from `PipelineRegistry::step_label(step_name)` — no
//! hardcoded labels or percentages in step bodies.
//!
//! Pass-1 (chunked mode) has additional per-chunk progress writes
//! that interpolate between `percent_start` and `percent_end`; those
//! are emitted inline in `llm_extract.rs` rather than through this
//! module because they consume the chunk-loop's running counters.
//!
//! ## Best-effort writes
//!
//! All helpers in this module trap errors with `.await.ok()` —
//! progress writes must never fail the step. A failed UPDATE leaves
//! the UI showing whatever the prior write set, which is the
//! correct degraded behavior. The `// best-effort:` comment on
//! every call site keeps the silent-discard rule satisfied
//! (CLAUDE.md Rule 5).

use sqlx::PgPool;
use tracing::instrument;

use crate::pipeline::context::AppContext;
use crate::repositories::pipeline_repository::documents;

/// Write the step's `percent_start` progress event.
///
/// Looks up the step's label entry in the registry. If no entry is
/// found (registry missing the step), the call is a no-op — same
/// degraded behavior as a failed DB write.
///
/// `step_name` is the dotted registry key (e.g. `"extract_text"`,
/// `"llm_extract_pass1"`). The frontend's `processing_step` column
/// receives this exact string.
#[instrument(skip(db, context), fields(doc_id, step_name))]
pub async fn write_start(db: &PgPool, context: &AppContext, doc_id: &str, step_name: &'static str) {
    if let Some(entry) = context.registry.step_label(step_name) {
        // best-effort: progress write — never fail the step on a DB error.
        documents::update_processing_progress(
            db,
            doc_id,
            step_name,
            &entry.label,
            0,
            0,
            0,
            entry.percent_start,
        )
        .await
        .ok();
    }
}

/// Write the step's `percent_end` progress event.
///
/// Same semantics as [`write_start`] but uses `percent_end` and a
/// `label` that the operator sees momentarily before the next step's
/// `write_start` overwrites it.
#[instrument(skip(db, context), fields(doc_id, step_name))]
pub async fn write_end(db: &PgPool, context: &AppContext, doc_id: &str, step_name: &'static str) {
    if let Some(entry) = context.registry.step_label(step_name) {
        // best-effort: progress write — never fail the step on a DB error.
        documents::update_processing_progress(
            db,
            doc_id,
            step_name,
            &entry.label,
            0,
            0,
            0,
            entry.percent_end,
        )
        .await
        .ok();
    }
}

/// Write a custom-label progress event at the step's `percent_end`.
///
/// Used by the Verify step, whose label embeds a `{grounding_pct}`
/// substitution that's only known after the step's work completes.
/// The caller resolves the substitution and passes the final string.
#[instrument(skip(db, context), fields(doc_id, step_name))]
pub async fn write_end_with_label(
    db: &PgPool,
    context: &AppContext,
    doc_id: &str,
    step_name: &'static str,
    label: &str,
) {
    if let Some(entry) = context.registry.step_label(step_name) {
        // best-effort: progress write — never fail the step on a DB error.
        documents::update_processing_progress(
            db,
            doc_id,
            step_name,
            label,
            0,
            0,
            0,
            entry.percent_end,
        )
        .await
        .ok();
    }
}
