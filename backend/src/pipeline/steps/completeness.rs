//! `pipeline/steps/completeness.rs` — entity-level completeness step.
//!
//! Terminal step of the `DocProcessing` FSM. For each approved
//! extraction item we compute the expected Neo4j id (via the same
//! helpers the HTTP handler uses), batch-verify node existence, and
//! batch-verify a Qdrant point per found node. The previous count-based
//! comparison is gone — see `COMPLETENESS_VERIFICATION_REDESIGN_v1.md`.
//!
//! ## check-fail IS step-fail
//!
//! The HTTP endpoint returns 200 OK with status "pass"/"warn"/"fail" in
//! the body because it's a diagnostic endpoint for a human. The pipeline
//! Step cannot do that — the FSM expects success or error. So missing
//! Document node or missing entity nodes convert to
//! `Err(CompletenessError::MissingNodes { … })` or
//! `Err(CompletenessError::MissingDocumentNode { … })`. Missing Qdrant
//! points remain a WARN: logged, but the step still succeeds and the
//! document still moves to PUBLISHED.
//!
//! ## Transitional `STATUS_PUBLISHED` write
//!
//! Preserved from the prior design — the 8-state legacy lifecycle still
//! ends in `"PUBLISHED"`. Phase 5 PS-B8 will coordinate the rename.
//!
//! ## Read-only step, no-op on_cancel
//!
//! This step reads Neo4j + Qdrant + Postgres, then writes exactly one
//! row to Postgres (the status update). No partial state to roll back.

use std::error::Error;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use colossus_pipeline::cancel::CancellationToken;
use colossus_pipeline::progress::ProgressReporter;
use colossus_pipeline::{Step, StepResult};

use crate::api::pipeline::completeness_helpers::{
    compute_expected_neo4j_ids, document_node_exists, verify_neo4j_nodes, verify_qdrant_points,
};
use crate::error::AppError;
use crate::models::document_status::STATUS_PUBLISHED;
use crate::pipeline::context::AppContext;
use crate::pipeline::task::DocProcessing;
use crate::repositories::pipeline_repository;

/// Completeness step state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Completeness {
    pub document_id: String,
}

/// Outcome of a successful pass through [`run_completeness`].
///
/// Consumed by:
/// - The legacy [`Completeness::execute`] thin wrapper — re-emits
///   the 4-key audit JSON via `progress.set_step_result(...)` so
///   `pipeline_steps.result_summary` stays byte-identical to the
///   pre-refactor shape.
/// - The Restate workflow handler (`step_completeness`) — builds a
///   journal summary string from these counters.
///
/// Mirrors the HTTP handler's response shape (without the full id
/// lists — those stay in the on-error `CompletenessError::MissingNodes`
/// variant where they're operator-actionable).
///
/// ## Why no `skipped_already_complete`
///
/// Completeness is naturally idempotent: it reads Neo4j + Qdrant +
/// Postgres and writes one row (`documents.status = "PUBLISHED"`,
/// idempotent — same value re-applied). Re-running on a PUBLISHED
/// document produces the same result. No short-circuit needed.
#[derive(Debug, Clone, Default)]
pub struct CompletenessResult {
    pub total_items: usize,
    pub nodes_verified: usize,
    pub points_verified: usize,
    pub points_missing: usize,
}

// ─────────────────────────────────────────────────────────────────────
// CompletenessError
// ─────────────────────────────────────────────────────────────────────

/// Failure modes for the Completeness step.
///
/// `MissingNodes` / `MissingDocumentNode` carry enough id detail in the
/// Display string so `pipeline_jobs.error_message` is actionable on its
/// own — the admin doesn't need a separate GET to `/completeness`.
///
/// Display strings omit the `#[source]` body (Kazlauskas G6).
#[derive(Debug, thiserror::Error)]
pub enum CompletenessError {
    #[error("Document '{doc_id}' not found")]
    DocumentNotFound { doc_id: String },

    #[error("No completed extraction run for document '{doc_id}'")]
    NoCompletedRun { doc_id: String },

    #[error("Document node missing in Neo4j for document '{doc_id}'")]
    MissingDocumentNode { doc_id: String },

    #[error(
        "Completeness failed for document '{doc_id}': {missing_count} of {total} \
         expected Neo4j nodes are missing. Missing ids: {ids}"
    )]
    MissingNodes {
        doc_id: String,
        missing_count: usize,
        total: usize,
        ids: String,
    },

    /// Helper-origin failure (Postgres or helper module). Stringly-typed
    /// message — same discipline as other step files.
    #[error("Helper failed for document '{doc_id}': {message}")]
    Helper { doc_id: String, message: String },
}

impl From<AppError> for CompletenessError {
    /// Convert errors from `completeness_helpers` (which return
    /// `AppError` for cross-path compatibility with the HTTP handler)
    /// into step-local failures routed through the Helper variant.
    fn from(err: AppError) -> Self {
        let message = match err {
            AppError::BadRequest { message, .. } => message,
            AppError::NotFound { message } => message,
            AppError::Unauthorized { message } => message,
            AppError::Forbidden { message } => message,
            AppError::Conflict { message, .. } => message,
            AppError::Internal { message } => message,
        };
        CompletenessError::Helper {
            doc_id: String::new(),
            message,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// Step impl
// ─────────────────────────────────────────────────────────────────────

#[async_trait]
impl Step<DocProcessing> for Completeness {
    const DEFAULT_RETRY_LIMIT: i32 = 3;
    const DEFAULT_RETRY_DELAY_SECS: u64 = 10;
    const DEFAULT_TIMEOUT_SECS: Option<u64> = Some(60);

    /// Thin wrapper over [`run_completeness`] — the clean business
    /// core that the Restate workflow handler also calls.
    ///
    /// Adds on top of the core:
    /// 1. **Pre-extraction cancel check** (legacy worker semantics).
    /// 2. **`progress.set_step_result(...)` audit JSON.** Re-emits
    ///    the 4-key shape the pre-refactor body wrote inline so
    ///    `pipeline_steps.result_summary` stays byte-identical.
    /// 3. **`StepResult::Done`** — Completeness is the terminal FSM
    ///    step on the legacy worker path. The Restate workflow body
    ///    sequences steps directly and does not see this routing.
    ///
    /// Note: the `documents.status = "PUBLISHED"` write happens
    /// inside the CORE function `run_completeness`, not here. Both
    /// legacy and Restate paths see the canonical status surface
    /// via the core.
    async fn execute(
        self,
        db: &PgPool,
        context: &AppContext,
        cancel: &CancellationToken,
        progress: &ProgressReporter,
    ) -> Result<StepResult<DocProcessing>, Box<dyn Error + Send + Sync>> {
        let start = Instant::now();

        if cancel.is_cancelled().await {
            return Err("Cancelled before completeness check".into());
        }

        let result = run_completeness(&self.document_id, db, context).await?;
        let duration_secs = start.elapsed().as_secs_f64();

        tracing::info!(
            doc_id = %self.document_id,
            duration_secs,
            total_items = result.total_items,
            nodes_verified = result.nodes_verified,
            points_verified = result.points_verified,
            points_missing = result.points_missing,
            "Completeness step complete — document PUBLISHED"
        );

        progress.set_step_result(serde_json::json!({
            "total_items": result.total_items,
            "nodes_verified": result.nodes_verified,
            "points_verified": result.points_verified,
            "points_missing": result.points_missing,
        }));

        Ok(StepResult::Done)
    }

    // on_cancel: read-only step, no partial state → trait-default no-op.
    // on_delete: trait default (Task::on_delete_current handles via
    // cleanup_all).
}

// ─────────────────────────────────────────────────────────────────────
// Core implementation (Restate-callable)
// ─────────────────────────────────────────────────────────────────────

/// Run the Completeness step — entity-level Neo4j + Qdrant verification.
///
/// For each approved extraction item: computes its expected Neo4j
/// id, batch-verifies the Neo4j node exists, batch-verifies a Qdrant
/// point exists per found node. Missing Document node or any missing
/// entity node → terminal error (operator must investigate before
/// the document can advance). Missing Qdrant points → WARN only
/// (re-indexing repairs).
///
/// On success, writes `documents.status = "PUBLISHED"` — the
/// terminal status of the legacy 8-state lifecycle. Both legacy and
/// Restate paths see this canonical surface via the core.
///
/// ## Idempotency
///
/// Naturally idempotent: read-only verifications + idempotent status
/// write (same value re-applied). Restate replay is safe; no
/// explicit short-circuit guard needed.
///
/// ## Cancellation
///
/// Does not poll a `CancellationToken`. The legacy worker wraps
/// `Step::execute` in `tokio::select!` with a `cancel_watcher`; the
/// Restate path kills the awaiting future via SDK abort.
pub async fn run_completeness(
    document_id: &str,
    db: &PgPool,
    context: &AppContext,
) -> Result<CompletenessResult, CompletenessError> {
    let doc_id = document_id;
    {
        // 1. Document exists in Postgres.
        let _document = pipeline_repository::get_document(db, doc_id)
            .await
            .map_err(|e| CompletenessError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("get_document: {e}"),
            })?
            .ok_or_else(|| CompletenessError::DocumentNotFound {
                doc_id: doc_id.to_string(),
            })?;

        // 2. Latest completed extraction run.
        let run_id = pipeline_repository::get_latest_completed_run(db, doc_id)
            .await
            .map_err(|e| CompletenessError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("get_latest_completed_run: {e}"),
            })?
            .ok_or_else(|| CompletenessError::NoCompletedRun {
                doc_id: doc_id.to_string(),
            })?;

        // 3. Approved extraction items.
        let items = pipeline_repository::get_approved_items_for_document(db, doc_id, run_id)
            .await
            .map_err(|e| CompletenessError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("get_approved_items: {e}"),
            })?;
        let total_items = items.len();

        // 4. Compute expected Neo4j ids.
        let expected: Vec<(i32, String)> = compute_expected_neo4j_ids(&items, doc_id);
        let expected_ids: Vec<String> = expected.iter().map(|(_, id)| id.clone()).collect();

        // 5. Document node — hard FAIL if missing.
        let document_node = document_node_exists(&context.graph, doc_id)
            .await
            .map_err(|e| helper_with_doc(doc_id, e))?;
        if !document_node {
            return Err(CompletenessError::MissingDocumentNode {
                doc_id: doc_id.to_string(),
            });
        }

        // 6. Batch Neo4j verification.
        let nodes_missing = verify_neo4j_nodes(&context.graph, &expected_ids)
            .await
            .map_err(|e| helper_with_doc(doc_id, e))?;
        let missing_set: std::collections::HashSet<&String> = nodes_missing.iter().collect();
        let found_node_ids: Vec<String> = expected_ids
            .iter()
            .filter(|id| !missing_set.contains(id))
            .cloned()
            .collect();
        let nodes_verified = found_node_ids.len();

        if !nodes_missing.is_empty() {
            let ids = nodes_missing.join(", ");
            return Err(CompletenessError::MissingNodes {
                doc_id: doc_id.to_string(),
                missing_count: nodes_missing.len(),
                total: expected_ids.len(),
                ids,
            });
        }

        // 7. Batch Qdrant verification — WARN only.
        let points_missing =
            verify_qdrant_points(&context.http_client, &context.qdrant_url, &found_node_ids)
                .await
                .map_err(|e| helper_with_doc(doc_id, e))?;
        let points_verified = found_node_ids.len() - points_missing.len();
        if !points_missing.is_empty() {
            tracing::warn!(
                doc_id = %doc_id,
                missing = points_missing.len(),
                "Completeness: {} Neo4j nodes have no Qdrant point — re-indexing would repair",
                points_missing.len()
            );
        }

        // 8. All nodes present → transition to PUBLISHED.
        //
        // NOTE: writes STATUS_PUBLISHED (legacy), not DOC_STATUS_COMPLETED
        // (tracker). See module doc — Phase 5 PS-B8 owns the rename.
        pipeline_repository::update_document_status(db, doc_id, STATUS_PUBLISHED)
            .await
            .map_err(|e| CompletenessError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("update_document_status: {e}"),
            })?;

        Ok(CompletenessResult {
            total_items,
            nodes_verified,
            points_verified,
            points_missing: points_missing.len(),
        })
    }
}

/// Attach `doc_id` to an `AppError` → `CompletenessError::Helper`
/// conversion. The `From<AppError>` impl can't fill `doc_id` on its
/// own — this helper does the final substitution at call sites.
fn helper_with_doc(doc_id: &str, err: AppError) -> CompletenessError {
    let mut e: CompletenessError = err.into();
    if let CompletenessError::Helper {
        doc_id: ref mut d, ..
    } = e
    {
        *d = doc_id.to_string();
    }
    e
}

// ─────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completeness_error_display_contains_doc_id() {
        let err = CompletenessError::DocumentNotFound {
            doc_id: "doc-xyz".to_string(),
        };
        assert!(format!("{err}").contains("doc-xyz"));
    }

    #[test]
    fn completeness_error_missing_nodes_display_enumerates_ids() {
        let err = CompletenessError::MissingNodes {
            doc_id: "doc-7".to_string(),
            missing_count: 2,
            total: 39,
            ids: "person-marie-awad, doc-awad:para:42".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("doc-7"));
        assert!(display.contains("2 of 39"));
        assert!(display.contains("person-marie-awad"));
        assert!(display.contains("doc-awad:para:42"));
    }

    #[test]
    fn completeness_error_missing_document_node_display() {
        let err = CompletenessError::MissingDocumentNode {
            doc_id: "doc-42".to_string(),
        };
        assert!(format!("{err}").contains("doc-42"));
    }
}
