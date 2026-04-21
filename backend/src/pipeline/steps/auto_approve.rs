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
use crate::repositories::pipeline_repository::{self, documents, review, steps};

/// AutoApprove step state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutoApprove {
    pub document_id: String,
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

    async fn execute(
        self,
        db: &PgPool,
        _context: &AppContext,
        cancel: &CancellationToken,
        _progress: &ProgressReporter,
    ) -> Result<StepResult<DocProcessing>, Box<dyn Error + Send + Sync>> {
        let start = Instant::now();
        let doc_id = self.document_id.clone();

        if cancel.is_cancelled().await {
            return Err("Cancelled before auto-approve".into());
        }

        let step_id = steps::record_step_start(
            db,
            &doc_id,
            "auto_approve",
            "pipeline",
            &serde_json::json!({}),
        )
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(
                doc_id = %doc_id, error = %e,
                "AutoApprove: record_step_start failed (non-fatal)"
            );
            0
        });

        // Guard: confirm the document exists.
        pipeline_repository::get_document(db, &doc_id)
            .await
            .map_err(|e| AutoApproveError::Helper {
                doc_id: doc_id.clone(),
                message: format!("get_document: {e}"),
            })?
            .ok_or_else(|| AutoApproveError::DocumentNotFound {
                doc_id: doc_id.clone(),
            })?;

        // Approve only items where grounding_status IN ('exact', 'normalized').
        let approved_count = review::bulk_approve(db, &doc_id, "pipeline", "grounded")
            .await
            .map_err(|source| AutoApproveError::BulkApproveFailed {
                doc_id: doc_id.clone(),
                source,
            })?;

        // Count remaining grounded+pending items (should be 0 immediately
        // after bulk_approve unless a race approved more in parallel).
        let remaining_pending = review::count_pending(db, &doc_id)
            .await
            .map_err(|source| AutoApproveError::CountPendingFailed {
                doc_id: doc_id.clone(),
                source,
            })?;

        if approved_count == 0 {
            tracing::warn!(
                doc_id = %doc_id,
                remaining_pending,
                "AutoApprove: zero items approved — zero items will reach Neo4j unless reviewed manually"
            );
        } else {
            tracing::info!(
                doc_id = %doc_id,
                approved_count,
                remaining_pending,
                "AutoApprove complete"
            );
        }

        // UI progress (non-critical).
        documents::update_processing_progress(
            db,
            &doc_id,
            "AutoApprove",
            &format!("Auto-approved {approved_count} grounded items"),
            0,
            0,
            0,
            0,
        )
        .await
        .ok();

        if cancel.is_cancelled().await {
            return Err("Cancelled after auto-approve".into());
        }

        let duration_secs = start.elapsed().as_secs_f64();
        tracing::info!(
            doc_id = %doc_id,
            duration_secs,
            approved_count,
            "AutoApprove step complete"
        );

        if step_id != 0 {
            let summary = serde_json::json!({
                "approved_count": approved_count,
                "remaining_pending": remaining_pending,
            });
            if let Err(e) = steps::record_step_complete(db, step_id, duration_secs, &summary).await
            {
                tracing::warn!(
                    doc_id = %doc_id, step_id, error = %e,
                    "AutoApprove: record_step_complete failed (non-fatal)"
                );
            }
        }

        Ok(StepResult::Next(DocProcessing::Ingest(Ingest {
            document_id: self.document_id,
        })))
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_approve_step_constants_match_spec() {
        assert_eq!(AutoApprove::DEFAULT_RETRY_LIMIT, 1);
        assert_eq!(AutoApprove::DEFAULT_RETRY_DELAY_SECS, 5);
        assert_eq!(AutoApprove::DEFAULT_TIMEOUT_SECS, Some(30));
    }

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
