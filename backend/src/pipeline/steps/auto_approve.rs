//! AutoApprove step: bulk-approves grounded extraction items via the
//! review repository's `bulk_approve` helper with `filter = "grounded"`.
//!
//! Items whose `grounding_status` is NOT in `('exact', 'normalized')` remain
//! in PENDING review_status and surface to the manual review queue — the
//! pipeline does not block on them; the Ingest step consumes only approved
//! items.

use std::error::Error;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use colossus_pipeline::cancel::CancellationToken;
use colossus_pipeline::progress::ProgressReporter;
use colossus_pipeline::{Step, StepResult};

use crate::pipeline::context::AppContext;
use crate::pipeline::steps::ingest::Ingest;
use crate::pipeline::task::DocProcessing;
use crate::repositories::pipeline_repository::{self, review};

/// AutoApprove step state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutoApprove {
    pub document_id: String,
}

/// Outcome of a successful pass through [`run_auto_approve`].
///
/// Consumed by:
/// - The legacy [`AutoApprove::execute`] thin wrapper — re-emits
///   the 2-key audit JSON via `progress.set_step_result(...)`
///   (`approved`, `pending_review`) so `pipeline_steps.result_summary`
///   stays byte-identical to the pre-refactor shape.
/// - The Restate workflow handler (`step_auto_approve`) — builds a
///   journal summary string from these counters.
///
/// ## Idempotency
///
/// `bulk_approve` only transitions items currently in PENDING with a
/// matching `grounding_status`. A second invocation finds nothing to
/// flip — `approved_count` will be `0` on replay. The replay-safe
/// behavior is intentional: we do NOT need a `skipped_already_complete`
/// flag because there is no terminal-vs-skipped semantic distinction
/// — both produce `approved_count = 0`.
#[derive(Debug, Clone, Default)]
pub struct AutoApproveResult {
    /// Items newly transitioned to APPROVED in this invocation. `0`
    /// on replay (the bulk_approve query filters by PENDING only).
    pub approved_count: u64,
    /// Items still PENDING after this invocation. The legacy wrapper
    /// renders this as the `pending_review` audit key. `i64` matches
    /// the repository return type
    /// ([`review::count_pending`](crate::repositories::pipeline_repository::review::count_pending)).
    pub pending_review_count: i64,
}

// ─────────────────────────────────────────────────────────────────────────
// AutoApproveError
// ─────────────────────────────────────────────────────────────────────────

/// Failure modes for the AutoApprove step.
///
/// Per-subsystem variants carry `doc_id` and thread the underlying `sqlx::Error`
/// via `#[source]`. Display strings deliberately omit `{source}` so log output
/// does not duplicate the inner message (Kazlauskas Guideline 6).
#[derive(Debug, thiserror::Error)]
pub enum AutoApproveError {
    #[error("Document '{doc_id}' not found")]
    DocumentNotFound { doc_id: String },

    #[error("Bulk approve failed for document '{doc_id}'")]
    BulkApproveFailed {
        doc_id: String,
        #[source]
        source: sqlx::Error,
    },

    #[error("count_pending failed for document '{doc_id}'")]
    CountPendingFailed {
        doc_id: String,
        #[source]
        source: sqlx::Error,
    },

    /// Helper-origin failure — `PipelineRepoError` is stringly-typed so we
    /// preserve only the message, not a source chain. Mirrors the
    /// `IngestError::Helper` convention.
    #[error("AutoApprove helper failed for document '{doc_id}': {message}")]
    Helper { doc_id: String, message: String },
}

// ─────────────────────────────────────────────────────────────────────────
// Step impl
// ─────────────────────────────────────────────────────────────────────────

#[async_trait]
impl Step<DocProcessing> for AutoApprove {
    const DEFAULT_RETRY_LIMIT: i32 = 1;
    const DEFAULT_RETRY_DELAY_SECS: u64 = 5;
    const DEFAULT_TIMEOUT_SECS: Option<u64> = Some(30);

    /// Thin wrapper over [`run_auto_approve`] — the clean business
    /// core that the Restate workflow handler also calls.
    ///
    /// Adds on top of the core:
    /// 1. **Pre / post cancel checks** (legacy worker semantics).
    /// 2. **`progress.set_step_result(...)` audit JSON.** Re-emits
    ///    the 2-key shape (`approved`, `pending_review`) the
    ///    pre-refactor body wrote inline so
    ///    `pipeline_steps.result_summary` stays byte-identical.
    /// 3. **FSM routing** to Ingest.
    async fn execute(
        self,
        db: &PgPool,
        context: &AppContext,
        cancel: &CancellationToken,
        progress: &ProgressReporter,
    ) -> Result<StepResult<DocProcessing>, Box<dyn Error + Send + Sync>> {
        let start = Instant::now();

        if cancel.is_cancelled().await {
            return Err("Cancelled before auto-approve".into());
        }

        let result = run_auto_approve(&self.document_id, db, context).await?;

        if cancel.is_cancelled().await {
            return Err("Cancelled after auto-approve".into());
        }

        let duration_secs = start.elapsed().as_secs_f64();
        tracing::info!(
            doc_id = %self.document_id,
            duration_secs,
            approved_count = result.approved_count,
            "AutoApprove step complete"
        );

        progress.set_step_result(serde_json::json!({
            "approved": result.approved_count,
            "pending_review": result.pending_review_count,
        }));

        Ok(StepResult::Next(DocProcessing::Ingest(Ingest {
            document_id: self.document_id,
        })))
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Core implementation (Restate-callable)
// ─────────────────────────────────────────────────────────────────────────

/// Run the AutoApprove step — bulk-approve grounded extraction items.
///
/// Calls `review::bulk_approve(... "grounded")` which transitions
/// every PENDING item whose `grounding_status` is `'exact'` or
/// `'normalized'` to APPROVED. Items with other grounding statuses
/// remain in PENDING and surface to the manual review queue.
///
/// Returns an [`AutoApproveResult`] with the newly-approved count
/// and the still-pending count. Both numbers feed the legacy
/// wrapper's audit JSON.
///
/// ## Idempotency
///
/// Naturally idempotent: re-running on a document whose grounded
/// items are already APPROVED produces `approved_count = 0` (the
/// bulk-approve SQL filters by PENDING only). Restate replay is
/// safe; no explicit short-circuit guard needed.
///
/// ## Cancellation
///
/// Does not poll a `CancellationToken`. The legacy worker wraps
/// `Step::execute` in `tokio::select!` with a `cancel_watcher`; the
/// Restate path kills the awaiting future via SDK abort.
pub async fn run_auto_approve(
    document_id: &str,
    db: &PgPool,
    context: &AppContext,
) -> Result<AutoApproveResult, AutoApproveError> {
    let doc_id = document_id;

    // UI progress — step started.
    crate::pipeline::step_progress::write_start(db, context, doc_id, "auto_approve").await;

    // Guard: confirm the document exists.
    pipeline_repository::get_document(db, doc_id)
        .await
        .map_err(|e| AutoApproveError::Helper {
            doc_id: doc_id.to_string(),
            message: format!("get_document: {e}"),
        })?
        .ok_or_else(|| AutoApproveError::DocumentNotFound {
            doc_id: doc_id.to_string(),
        })?;

    // Approve only items where grounding_status IN ('exact', 'normalized').
    let approved_count = review::bulk_approve(db, doc_id, "pipeline", "grounded")
        .await
        .map_err(|source| AutoApproveError::BulkApproveFailed {
            doc_id: doc_id.to_string(),
            source,
        })?;

    // Count remaining grounded+pending items (should be 0 immediately
    // after bulk_approve unless a race approved more in parallel).
    let pending_review_count = review::count_pending(db, doc_id).await.map_err(|source| {
        AutoApproveError::CountPendingFailed {
            doc_id: doc_id.to_string(),
            source,
        }
    })?;

    if approved_count == 0 {
        tracing::warn!(
            doc_id = %doc_id,
            pending_review_count,
            "AutoApprove: zero items approved — zero items will reach Neo4j unless reviewed manually"
        );
    } else {
        tracing::info!(
            doc_id = %doc_id,
            approved_count,
            pending_review_count,
            "AutoApprove complete"
        );
    }

    // UI progress — step complete. Workflow-level percent_end (B5
    // fix: no longer drops the bar to 0%). The label comes from
    // the registry; the auto-approval count is observable via the
    // Restate journal summary and the review-tab counters.
    crate::pipeline::step_progress::write_end(db, context, doc_id, "auto_approve").await;

    Ok(AutoApproveResult {
        approved_count,
        pending_review_count,
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_approve_error_document_not_found_display_contains_doc_id() {
        let err = AutoApproveError::DocumentNotFound {
            doc_id: "missing-doc-99".to_string(),
        };
        assert!(format!("{err}").contains("missing-doc-99"));
    }

    #[test]
    fn auto_approve_error_bulk_approve_display_excludes_source_text() {
        const UNIQUE_INNER: &str = "UNIQUE_AUTOAPPROVE_INNER";
        let err = AutoApproveError::BulkApproveFailed {
            doc_id: "doc-7".to_string(),
            source: sqlx::Error::Configuration(UNIQUE_INNER.into()),
        };
        let display = format!("{err}");
        assert!(display.contains("doc-7"), "got: {display}");
        assert!(
            !display.contains(UNIQUE_INNER),
            "Display must not duplicate inner source (Kazlauskas 6); got: {display}"
        );
    }
}
