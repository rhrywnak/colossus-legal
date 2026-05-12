//! Re-verify & Sync endpoint: chains verify → auto-approve → ingest-delta
//! in a single operation for post-publish documents.
//!
//! ## Why this endpoint exists
//!
//! After a document is fully processed and published, the verify endpoint's
//! status guard (EXTRACTED/VERIFIED only) prevents re-verification. When
//! the verifier is improved (e.g., cross-page matching), there's no way to
//! apply improvements to already-processed documents. This endpoint bypasses
//! that guard and chains all three downstream operations so the user gets
//! one-click re-verification with immediate graph sync.

use std::collections::HashMap;

use axum::{extract::Path, extract::State, Json};
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::models::document_status::{
    STATUS_COMPLETED, STATUS_INDEXED, STATUS_INGESTED, STATUS_PUBLISHED,
};
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository::{self, review as review_repo};
use crate::state::AppState;

// ── Response DTOs ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ReverifySyncResponse {
    pub document_id: String,
    pub verify_results: VerifyResults,
    pub auto_approve_results: AutoApproveResults,
    /// Present only if ingest-delta ran. Absent when there were no
    /// newly-approved items to write to the graph.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ingest_delta_results: Option<IngestDeltaResults>,
    /// Non-nil when a later phase failed after an earlier phase succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_error: Option<String>,
    pub duration_secs: f64,
}

#[derive(Debug, Serialize)]
pub struct VerifyResults {
    pub total_items: usize,
    pub exact: usize,
    pub normalized: usize,
    pub not_found: usize,
    pub derived: usize,
    pub derived_invalid: usize,
    pub unverified: usize,
    /// How many items had their grounding_status change from the
    /// previous value. Tells the user whether re-verification
    /// actually did anything (e.g., cross-page fix applied).
    pub changed: usize,
}

#[derive(Debug, Serialize)]
pub struct AutoApproveResults {
    pub newly_approved: u64,
}

#[derive(Debug, Serialize)]
pub struct IngestDeltaResults {
    pub written_to_graph: usize,
}

// ── Status guard ───────────────────────────────────────────────

/// Whether a document with the given status can be re-verified and synced.
///
/// Only post-ingest statuses are allowed. Pre-ingest documents should use
/// the normal pipeline flow (verify endpoint with its own guards).
///
/// ## Rust Learning: pure function as a policy gate
///
/// Extracting the status check into a pure function makes it testable
/// without standing up an Axum handler or database connection. The handler
/// calls this; unit tests call it directly with every status variant.
pub(crate) fn is_reverify_sync_allowed(status: &str) -> bool {
    matches!(
        status,
        STATUS_INGESTED | STATUS_INDEXED | STATUS_PUBLISHED | STATUS_COMPLETED
    )
}

// ── Handler ────────────────────────────────────────────────────

/// POST /api/admin/pipeline/documents/:id/reverify-sync
///
/// Chains three operations in sequence:
/// 1. Re-verify all extraction items against canonical text
/// 2. Auto-approve any pending items that now have grounded status
/// 3. Write newly-approved items to Neo4j via ingest-delta
///
/// Returns a combined response summarizing what each phase did.
/// If a later phase fails, earlier results are still returned with
/// a `partial_error` field describing what went wrong.
pub async fn reverify_sync_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<ReverifySyncResponse>, AppError> {
    require_admin(&user)?;
    let start = std::time::Instant::now();
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST reverify-sync");

    // Status guard
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    if !is_reverify_sync_allowed(&document.status) {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot re-verify: status is '{}'. \
                 Re-verify & Sync is for post-ingest documents \
                 ({STATUS_INGESTED}, {STATUS_INDEXED}, {STATUS_PUBLISHED}, {STATUS_COMPLETED}).",
                document.status
            ),
            details: serde_json::json!({ "status": document.status }),
        });
    }

    // ── Phase 1: Snapshot + Re-verify ──────────────────────────

    // Snapshot current grounding_status per item so we can count changes.
    let before_items = pipeline_repository::get_all_items(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to snapshot items before re-verify: {e}"),
        })?;
    let before_status: HashMap<i32, Option<String>> = before_items
        .iter()
        .map(|item| (item.id, item.grounding_status.clone()))
        .collect();

    let verify_response =
        super::verify::run_verify(&state, &doc_id, &user.username).await?;

    // Count how many items changed grounding_status.
    let after_items = pipeline_repository::get_all_items(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to load items after re-verify: {e}"),
        })?;
    let changed = after_items
        .iter()
        .filter(|item| {
            before_status
                .get(&item.id)
                .map_or(true, |old| *old != item.grounding_status)
        })
        .count();

    let verify_results = VerifyResults {
        total_items: verify_response.total_items,
        exact: verify_response.grounded_exact,
        normalized: verify_response.grounded_normalized,
        not_found: verify_response.not_found,
        derived: verify_response.derived,
        derived_invalid: verify_response.derived_invalid,
        unverified: verify_response.unverified,
        changed,
    };

    tracing::info!(
        doc_id = %doc_id,
        total = verify_results.total_items,
        changed,
        exact = verify_results.exact,
        normalized = verify_results.normalized,
        "Re-verify phase complete"
    );

    // ── Phase 2: Auto-approve ──────────────────────────────────

    let newly_approved = match review_repo::bulk_approve(
        &state.pipeline_pool,
        &doc_id,
        &user.username,
        "grounded",
    )
    .await
    {
        Ok(count) => {
            tracing::info!(doc_id = %doc_id, newly_approved = count, "Auto-approve phase complete");
            count
        }
        Err(e) => {
            let err_msg = format!("Auto-approve failed: {e}");
            tracing::error!(doc_id = %doc_id, error = %e, "Re-verify auto-approve phase failed");
            return Ok(Json(ReverifySyncResponse {
                document_id: doc_id,
                verify_results,
                auto_approve_results: AutoApproveResults { newly_approved: 0 },
                ingest_delta_results: None,
                partial_error: Some(err_msg),
                duration_secs: start.elapsed().as_secs_f64(),
            }));
        }
    };

    let auto_approve_results = AutoApproveResults { newly_approved };

    // ── Phase 3: Ingest delta ──────────────────────────────────

    let ingest_delta_results =
        match super::ingest::run_ingest_delta(&state, &doc_id, &user.username).await {
            Ok(delta) => {
                let total = delta.nodes_written.total;
                tracing::info!(
                    doc_id = %doc_id,
                    written_to_graph = total,
                    "Ingest-delta phase complete"
                );
                Some(IngestDeltaResults {
                    written_to_graph: total,
                })
            }
            Err(e) => {
                let err_msg = format!("Ingest-delta failed: {e:?}");
                tracing::error!(
                    doc_id = %doc_id,
                    error = ?e,
                    "Re-verify ingest-delta phase failed"
                );
                return Ok(Json(ReverifySyncResponse {
                    document_id: doc_id,
                    verify_results,
                    auto_approve_results,
                    ingest_delta_results: None,
                    partial_error: Some(err_msg),
                    duration_secs: start.elapsed().as_secs_f64(),
                }));
            }
        };

    let duration = start.elapsed().as_secs_f64();

    log_admin_action(
        &state.audit_repo,
        &user.username,
        "pipeline.document.reverify_sync",
        Some("document"),
        Some(&doc_id),
        Some(serde_json::json!({
            "changed": changed,
            "newly_approved": newly_approved,
            "written_to_graph": ingest_delta_results.as_ref().map(|r| r.written_to_graph),
            "duration_secs": format!("{duration:.2}"),
        })),
    )
    .await;

    tracing::info!(
        doc_id = %doc_id,
        changed,
        newly_approved,
        duration_secs = format!("{duration:.2}"),
        "Re-verify & Sync complete"
    );

    Ok(Json(ReverifySyncResponse {
        document_id: doc_id,
        verify_results,
        auto_approve_results,
        ingest_delta_results,
        partial_error: None,
        duration_secs: duration,
    }))
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::document_status::*;

    /// Verify the status guard accepts all post-ingest statuses.
    #[test]
    fn test_reverify_sync_allowed_post_ingest_statuses() {
        let allowed = [
            STATUS_INGESTED,
            STATUS_INDEXED,
            STATUS_PUBLISHED,
            STATUS_COMPLETED,
        ];
        for status in &allowed {
            assert!(
                is_reverify_sync_allowed(status),
                "Re-verify & Sync must be allowed for status '{status}'"
            );
        }
    }

    /// Verify the status guard rejects all pre-ingest statuses.
    /// These documents should use the normal pipeline flow.
    #[test]
    fn test_reverify_sync_rejected_pre_ingest_statuses() {
        let rejected = [
            STATUS_NEW,
            STATUS_UPLOADED,
            STATUS_PROCESSING,
            STATUS_CLASSIFIED,
            STATUS_TEXT_EXTRACTED,
            STATUS_EXTRACTED,
            STATUS_VERIFIED,
            STATUS_IN_REVIEW,
            STATUS_APPROVED,
            STATUS_FAILED,
            STATUS_CANCELLED,
        ];
        for status in &rejected {
            assert!(
                !is_reverify_sync_allowed(status),
                "Re-verify & Sync must NOT be allowed for status '{status}'"
            );
        }
    }

    /// Verify that unknown/future statuses are rejected by default.
    #[test]
    fn test_reverify_sync_rejected_unknown_status() {
        assert!(!is_reverify_sync_allowed("SOME_FUTURE_STATUS"));
        assert!(!is_reverify_sync_allowed(""));
    }
}
