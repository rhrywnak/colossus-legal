//! Review endpoints for the extraction pipeline.
//!
//! This module was split out of a single oversized `review.rs` (Rule 17, no
//! module over 300 lines) into focused submodules, each re-exported here so the
//! router's `review::<handler>` paths in `super::mod` keep resolving unchanged:
//!
//!   - [`actions`]       — per-item decisions: approve, reject, item history
//!   - [`revisions`]     — per-item revisions/reversals: edit, unapprove, unreject
//!   - [`document_ops`]  — document-level ops: revert-ingest, reprocess, bulk-approve
//!
//! The request/response DTOs, the two shared helpers
//! ([`check_not_post_ingest`], [`is_rejection_allowed`]), and the
//! rejection-policy tests live here because every submodule depends on them.

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::models::document_status::{
    STATUS_COMPLETED, STATUS_INDEXED, STATUS_INGESTED, STATUS_PUBLISHED,
};
use crate::repositories::pipeline_repository;
use crate::state::AppState;

use colossus_extract::EntityCategory;

mod actions;
mod document_ops;
mod revisions;

// Re-export every handler so the router in `super::mod` continues to reference
// them as `review::<handler>` after the split.
pub use actions::{approve_handler, item_history_handler, reject_handler};
pub use document_ops::{bulk_approve_handler, reprocess_handler, revert_ingest_handler};
pub use revisions::{edit_handler, unapprove_handler, unreject_handler};

// ── Request DTOs ────────────────────────────────────────────────
//
// `deny_unknown_fields` on every request DTO (Rule 1, no silent failures): a
// client that sends a misspelled or stray key gets a 422 telling them which
// field was unexpected, instead of the typo being silently dropped and the
// request behaving as if the field were absent.

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApproveRequest {
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RejectRequest {
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditRequest {
    pub grounded_page: Option<i32>,
    pub verbatim_quote: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BulkApproveRequest {
    pub filter: String,
}

// ── Response DTOs ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ReviewResponse {
    pub id: i32,
    pub review_status: String,
    pub reviewed_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounded_page: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cascade_warning: Option<CascadeWarning>,
}

#[derive(Debug, Serialize)]
pub struct CascadeWarning {
    pub affected_relationships: i64,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct BulkApproveResponse {
    pub document_id: String,
    pub approved_count: u64,
    pub skipped_ungrounded: i64,
    pub remaining_pending: i64,
}

#[derive(Debug, Serialize)]
pub struct RevertIngestResponse {
    pub document_id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ReprocessResponse {
    pub document_id: String,
    pub status: String,
    pub message: String,
}

// ── Shared helpers ─────────────────────────────────────────────

/// Check that the document is not post-ingest. Returns Conflict error if it is.
///
/// `pub(super)` so the [`revisions`] submodule (unapprove/unreject) can call it
/// while keeping it off the crate's public surface.
pub(super) async fn check_not_post_ingest(
    state: &AppState,
    document_id: &str,
    action: &str,
) -> Result<(), AppError> {
    let document = pipeline_repository::get_document(&state.pipeline_pool, document_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!(
                "Failed to fetch document '{document_id}' while checking post-ingest status: {e}"
            ),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{document_id}' not found"),
        })?;

    if matches!(
        document.status.as_str(),
        STATUS_INGESTED | STATUS_INDEXED | STATUS_PUBLISHED | STATUS_COMPLETED
    ) {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot {action}: document is post-ingest (status: {}). Revert ingest first.",
                document.status
            ),
            details: serde_json::json!({"status": document.status}),
        });
    }
    Ok(())
}

// ── Rejection policy ───────────────────────────────────────────

/// Whether an entity with the given category can be rejected.
///
/// All entity categories are rejectable — Foundation, Structural,
/// Evidence, and Reference alike. This was not always the case:
/// an earlier version blocked Foundation entities, but that made
/// it impossible to remove bad Harms / LegalCounts / Elements
/// without re-extracting the entire document.
///
/// ## Rust Learning: pure function as a policy gate
///
/// Extracting this decision into a pure function (no IO, no state)
/// lets us test the rejection policy without needing a database or
/// an Axum handler. The handler calls this before proceeding; tests
/// call it directly with every category variant.
pub(super) fn is_rejection_allowed(_category: &EntityCategory) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that rejection is allowed for every entity category.
    ///
    /// This test would FAIL if a Foundation guard were re-introduced
    /// (i.e., `is_rejection_allowed` returned false for Foundation).
    /// It exists to prevent regression of the bug where the Reject
    /// button rendered but the backend blocked the API call.
    #[test]
    fn test_rejection_allowed_for_all_categories() {
        let categories = [
            EntityCategory::Foundation,
            EntityCategory::Structural,
            EntityCategory::Evidence,
            EntityCategory::Reference,
        ];
        for cat in &categories {
            assert!(
                is_rejection_allowed(cat),
                "Rejection must be allowed for {:?} — all entity categories are rejectable",
                cat
            );
        }
    }

    /// Foundation entities specifically must be rejectable.
    /// This is the exact scenario that was broken: Harm, LegalCount,
    /// and Element entities had a Reject button but clicking it
    /// returned a 400 error that the frontend silently swallowed.
    #[test]
    fn test_rejection_allowed_for_foundation() {
        assert!(
            is_rejection_allowed(&EntityCategory::Foundation),
            "Foundation entities (Harm, LegalCount, Element) must be rejectable"
        );
    }
}
