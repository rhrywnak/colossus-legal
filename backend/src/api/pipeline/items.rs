//! GET /api/admin/pipeline/documents/:id/items — list extraction items.
//! Supports pagination and optional filtering by review_status, entity_type,
//! and grounding_status. Returns items from the latest completed run.
//!
//! Each item includes `category`, `available_actions`, and `locked` fields
//! computed from the extraction schema and document status. Legacy boolean
//! fields (`can_approve`, `can_reject`, `can_edit`) are derived from
//! `available_actions` for backward compatibility.

use std::collections::HashMap;
use std::path::Path;

use axum::{extract::Path as AxumPath, extract::Query, extract::State, Json};
use colossus_extract::EntityCategory;
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::pipeline_repository::{self, review as review_repo};
use crate::state::AppState;

use review_repo::ReviewItemRow;

#[derive(Debug, Deserialize)]
pub struct ListItemsParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub review_status: Option<String>,
    pub entity_type: Option<String>,
    pub grounding_status: Option<String>,
}

/// An extraction item with computed action permission fields.
#[derive(Debug, Serialize)]
pub struct ReviewItemResponse {
    #[serde(flatten)]
    pub item: ReviewItemRow,
    /// Entity category from the schema: "foundation", "structural", "evidence", "reference"
    pub category: String,
    /// Actions available for this item given its category, status, and lock state.
    pub available_actions: Vec<String>,
    /// True if the document is post-ingest (items cannot be modified).
    pub locked: bool,
    /// Whether this item can be approved (legacy — derived from available_actions).
    pub can_approve: bool,
    /// Whether this item can be rejected (legacy — derived from available_actions).
    pub can_reject: bool,
    /// Whether this item can be edited (legacy — derived from available_actions).
    pub can_edit: bool,
}

/// Summary counts for the review panel header.
#[derive(Debug, Serialize)]
pub struct ReviewSummary {
    pub pending: i64,
    pub approved: i64,
    pub rejected: i64,
    pub edited: i64,
    pub total: i64,
    /// Count of items approved/edited in PG that have not yet been written
    /// to Neo4j (i.e., `neo4j_node_id IS NULL`). Non-zero only on post-
    /// ingest documents — on pre-ingest docs the Ingest step writes
    /// everything in one shot, so this is always 0.
    ///
    /// Drives the "Write N approved items to graph" button in the Review
    /// panel after Phase 3 lands. Zero means no delta work pending.
    pub pending_graph_write: i64,
}

#[derive(Debug, Serialize)]
pub struct ListItemsResponse {
    pub document_id: String,
    pub items: Vec<ReviewItemResponse>,
    pub summary: ReviewSummary,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

/// Compute available actions based on entity category, review status, and lock state.
///
/// ## Rust Learning: Enum matching for business rules
///
/// The category determines which actions are structurally valid. The review
/// status determines reversibility actions. The lock state overrides everything.
fn compute_available_actions(
    category: &EntityCategory,
    review_status: &str,
    locked: bool,
) -> Vec<String> {
    if locked {
        return vec![];
    }

    let status = review_status.to_lowercase();
    let mut actions = Vec::new();

    // Primary actions based on category (only for pending items)
    if status == "pending" || status.is_empty() {
        match category {
            EntityCategory::Foundation => {
                actions.push("confirm".to_string());
                actions.push("edit".to_string());
            }
            EntityCategory::Structural | EntityCategory::Evidence => {
                actions.push("approve".to_string());
                actions.push("reject".to_string());
                actions.push("edit".to_string());
            }
            EntityCategory::Reference => {
                actions.push("approve".to_string());
                actions.push("reject".to_string());
            }
        }
    }

    // Reversibility actions
    if status == "approved" || status == "edited" {
        actions.push("unapprove".to_string());
    }
    if status == "rejected" {
        actions.push("unreject".to_string());
    }

    actions
}

/// Convert EntityCategory to its string representation.
fn category_to_string(cat: &EntityCategory) -> &'static str {
    match cat {
        EntityCategory::Foundation => "foundation",
        EntityCategory::Structural => "structural",
        EntityCategory::Evidence => "evidence",
        EntityCategory::Reference => "reference",
    }
}

/// Load entity category map from schema. Returns empty map on failure
/// (all items default to Evidence).
async fn load_category_map(state: &AppState, doc_id: &str) -> HashMap<String, EntityCategory> {
    let pipe_config =
        match pipeline_repository::get_pipeline_config(&state.pipeline_pool, doc_id).await {
            Ok(Some(cfg)) => cfg,
            _ => return HashMap::new(),
        };

    let schema_path = format!(
        "{}/{}",
        state.config.extraction_schema_dir, pipe_config.schema_file
    );
    let schema = match colossus_extract::ExtractionSchema::from_file(Path::new(&schema_path)) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(doc_id = %doc_id, error = %e, "Failed to load schema for category lookup — defaulting to Evidence");
            return HashMap::new();
        }
    };

    schema
        .entity_types
        .iter()
        .map(|et| (et.name.clone(), et.category.clone()))
        .collect()
}

/// Check if a document status is post-ingest (Neo4j has been written).
fn is_post_ingest(status: &str) -> bool {
    matches!(status, "INGESTED" | "INDEXED" | "PUBLISHED" | "COMPLETED")
}

/// Item-level review statuses that are locked once the document is post-ingest.
///
/// - `approved`: item was written to Neo4j by Ingest; flipping it back without
///   reverting the graph would leave Neo4j and Postgres inconsistent.
/// - `rejected`: preserves existing UX — `check_not_post_ingest` in
///   `review.rs` gates `unreject` on post-ingest documents, so surfacing an
///   unreject button here would just produce a 409.
///
/// `pending` and `edited` are intentionally NOT locked:
/// - `pending` items were never ingested — safe to approve/reject/edit.
/// - `edited` items are excluded from the Ingest SQL filter
///   (`get_approved_items_for_document` matches `review_status = 'approved'`
///   only), so they too are not in Neo4j. Leaving them unlocked lets the
///   user finish the edit → approve flow post-publish.
fn is_post_ingest_locked_status(review_status: &str) -> bool {
    matches!(review_status.to_lowercase().as_str(), "approved" | "rejected")
}

/// GET /api/admin/pipeline/documents/:id/items
pub async fn list_items_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
    Query(params): Query<ListItemsParams>,
) -> Result<Json<ListItemsResponse>, AppError> {
    require_admin(&user)?;

    // Find latest completed run for this document
    let run_id = pipeline_repository::get_latest_completed_run(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("No completed extraction run for document '{doc_id}'"),
        })?;

    // Load document status for lock check
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;
    let doc_is_post_ingest = is_post_ingest(&document.status);

    // Load category map from schema
    let category_map = load_category_map(&state, &doc_id).await;

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).min(200);
    let offset = ((page - 1) * per_page) as i64;
    let limit = per_page as i64;

    let review_status = params.review_status.as_deref();
    let entity_type = params.entity_type.as_deref();
    let grounding_status = params.grounding_status.as_deref();

    let total = review_repo::count_items(
        &state.pipeline_pool,
        run_id,
        review_status,
        entity_type,
        grounding_status,
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Count query failed: {e}"),
    })?;

    let items = review_repo::list_items(
        &state.pipeline_pool,
        run_id,
        review_status,
        entity_type,
        grounding_status,
        limit,
        offset,
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("List query failed: {e}"),
    })?;

    // Compute summary counts (unfiltered — always show total picture)
    let total_all = review_repo::count_items(&state.pipeline_pool, run_id, None, None, None)
        .await
        .unwrap_or(0);
    let pending_count =
        review_repo::count_items(&state.pipeline_pool, run_id, Some("pending"), None, None)
            .await
            .unwrap_or(0);
    let approved_count =
        review_repo::count_items(&state.pipeline_pool, run_id, Some("approved"), None, None)
            .await
            .unwrap_or(0);
    let rejected_count =
        review_repo::count_items(&state.pipeline_pool, run_id, Some("rejected"), None, None)
            .await
            .unwrap_or(0);
    let edited_count =
        review_repo::count_items(&state.pipeline_pool, run_id, Some("edited"), None, None)
            .await
            .unwrap_or(0);

    // Pending graph writes (approved/edited in PG, not yet in Neo4j).
    // Only meaningful on post-ingest documents; skip the query otherwise
    // to keep pre-ingest list responses cheap.
    let pending_graph_write = if doc_is_post_ingest {
        pipeline_repository::count_items_pending_graph_write(&state.pipeline_pool, &doc_id)
            .await
            .unwrap_or(0)
    } else {
        0
    };

    // Enrich items with category, available_actions, locked, and legacy booleans.
    // `locked` is per-item: the doc must be post-ingest AND the item's review
    // status must be in the lock set (approved/rejected). Pending/edited items
    // remain actionable on post-ingest docs so the user can finish review.
    let enriched_items: Vec<ReviewItemResponse> = items
        .into_iter()
        .map(|item| {
            let category = category_map
                .get(&item.entity_type)
                .unwrap_or(&EntityCategory::Evidence);
            let locked =
                doc_is_post_ingest && is_post_ingest_locked_status(&item.review_status);
            let actions = compute_available_actions(category, &item.review_status, locked);
            let can_approve = actions.iter().any(|a| a == "approve" || a == "confirm");
            let can_reject = actions.iter().any(|a| a == "reject");
            let can_edit = actions.iter().any(|a| a == "edit");
            let category_str = category_to_string(category).to_string();
            ReviewItemResponse {
                item,
                category: category_str,
                available_actions: actions,
                locked,
                can_approve,
                can_reject,
                can_edit,
            }
        })
        .collect();

    let total_pages = if total == 0 {
        1
    } else {
        (total as u32).div_ceil(per_page)
    };

    Ok(Json(ListItemsResponse {
        document_id: doc_id,
        items: enriched_items,
        summary: ReviewSummary {
            pending: pending_count,
            approved: approved_count,
            rejected: rejected_count,
            edited: edited_count,
            total: total_all,
            pending_graph_write,
        },
        total,
        page,
        per_page,
        total_pages,
    }))
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_item_actions_pending() {
        let actions = compute_available_actions(&EntityCategory::Evidence, "pending", false);
        assert!(actions.contains(&"approve".to_string()));
        assert!(actions.contains(&"reject".to_string()));
        assert!(actions.contains(&"edit".to_string()));
    }

    #[test]
    fn compute_item_actions_empty() {
        let actions = compute_available_actions(&EntityCategory::Evidence, "", false);
        assert!(actions.contains(&"approve".to_string()));
        assert!(actions.contains(&"reject".to_string()));
        assert!(actions.contains(&"edit".to_string()));
    }

    #[test]
    fn compute_item_actions_approved() {
        let actions = compute_available_actions(&EntityCategory::Evidence, "approved", false);
        assert!(!actions.contains(&"approve".to_string()));
        assert!(!actions.contains(&"reject".to_string()));
        assert!(actions.contains(&"unapprove".to_string()));
    }

    #[test]
    fn compute_item_actions_rejected() {
        let actions = compute_available_actions(&EntityCategory::Evidence, "rejected", false);
        assert!(!actions.contains(&"approve".to_string()));
        assert!(actions.contains(&"unreject".to_string()));
    }

    #[test]
    fn compute_item_actions_edited() {
        let actions = compute_available_actions(&EntityCategory::Evidence, "edited", false);
        assert!(actions.contains(&"unapprove".to_string()));
    }

    #[test]
    fn compute_item_actions_case_insensitive() {
        let actions = compute_available_actions(&EntityCategory::Evidence, "Pending", false);
        assert!(actions.contains(&"approve".to_string()));
        let actions = compute_available_actions(&EntityCategory::Evidence, "PENDING", false);
        assert!(actions.contains(&"approve".to_string()));
    }

    #[test]
    fn compute_item_actions_foundation_pending() {
        let actions = compute_available_actions(&EntityCategory::Foundation, "pending", false);
        assert!(actions.contains(&"confirm".to_string()));
        assert!(actions.contains(&"edit".to_string()));
        assert!(!actions.contains(&"approve".to_string()));
        assert!(!actions.contains(&"reject".to_string()));
    }

    #[test]
    fn compute_item_actions_reference_pending() {
        let actions = compute_available_actions(&EntityCategory::Reference, "pending", false);
        assert!(actions.contains(&"approve".to_string()));
        assert!(actions.contains(&"reject".to_string()));
        assert!(!actions.contains(&"edit".to_string()));
    }

    #[test]
    fn compute_item_actions_locked() {
        let actions = compute_available_actions(&EntityCategory::Evidence, "pending", true);
        assert!(actions.is_empty());
    }

    #[test]
    fn compute_item_actions_locked_approved() {
        let actions = compute_available_actions(&EntityCategory::Evidence, "approved", true);
        assert!(actions.is_empty());
    }

    // ── Post-ingest lock set ─────────────────────────────────────

    #[test]
    fn post_ingest_lock_set_contains_approved_and_rejected() {
        assert!(is_post_ingest_locked_status("approved"));
        assert!(is_post_ingest_locked_status("rejected"));
    }

    #[test]
    fn post_ingest_lock_set_excludes_pending_and_edited() {
        // Pending items were never ingested — they must stay actionable
        // on PUBLISHED docs so the user can review what the pipeline couldn't
        // auto-approve.
        assert!(!is_post_ingest_locked_status("pending"));
        assert!(!is_post_ingest_locked_status(""));
        // Edited items are excluded from get_approved_items_for_document
        // (SQL filters review_status = 'approved' only), so they are NOT in
        // Neo4j even on a "published" doc. Treat them like pending.
        assert!(!is_post_ingest_locked_status("edited"));
    }

    #[test]
    fn post_ingest_lock_set_is_case_insensitive() {
        assert!(is_post_ingest_locked_status("APPROVED"));
        assert!(is_post_ingest_locked_status("Rejected"));
    }

    #[test]
    fn post_ingest_lock_set_rejects_unknown_status() {
        // Defensive: unknown statuses shouldn't accidentally count as locked.
        assert!(!is_post_ingest_locked_status("garbage"));
        assert!(!is_post_ingest_locked_status("in_review"));
    }

    // ── is_post_ingest (document-level) ──────────────────────────

    #[test]
    fn is_post_ingest_true_for_ingested_indexed_published_completed() {
        assert!(is_post_ingest("INGESTED"));
        assert!(is_post_ingest("INDEXED"));
        assert!(is_post_ingest("PUBLISHED"));
        assert!(is_post_ingest("COMPLETED"));
    }

    #[test]
    fn is_post_ingest_false_for_pre_ingest_statuses() {
        assert!(!is_post_ingest("NEW"));
        assert!(!is_post_ingest("EXTRACTED"));
        assert!(!is_post_ingest("VERIFIED"));
        assert!(!is_post_ingest("TEXT_EXTRACTED"));
    }

    // ── Integration: (doc_status, review_status) → effective lock ─
    //
    // These exercise the exact expression the handler builds:
    //   locked = doc_is_post_ingest && is_post_ingest_locked_status(rs)
    // Table-driven so the matrix is visible in one place.

    #[test]
    fn effective_lock_matrix() {
        #[derive(Debug)]
        struct Case {
            doc_status: &'static str,
            review_status: &'static str,
            expect_locked: bool,
        }
        let cases = [
            // Pre-ingest docs — nothing is locked regardless of item status.
            Case { doc_status: "VERIFIED", review_status: "pending",  expect_locked: false },
            Case { doc_status: "VERIFIED", review_status: "approved", expect_locked: false },
            Case { doc_status: "VERIFIED", review_status: "rejected", expect_locked: false },
            Case { doc_status: "VERIFIED", review_status: "edited",   expect_locked: false },
            // Post-ingest docs — approved and rejected lock, pending and edited do not.
            Case { doc_status: "PUBLISHED", review_status: "pending",  expect_locked: false },
            Case { doc_status: "PUBLISHED", review_status: "approved", expect_locked: true  },
            Case { doc_status: "PUBLISHED", review_status: "rejected", expect_locked: true  },
            Case { doc_status: "PUBLISHED", review_status: "edited",   expect_locked: false },
            Case { doc_status: "INGESTED",  review_status: "pending",  expect_locked: false },
            Case { doc_status: "INGESTED",  review_status: "approved", expect_locked: true  },
        ];
        for c in cases {
            let locked = is_post_ingest(c.doc_status) && is_post_ingest_locked_status(c.review_status);
            assert_eq!(
                locked, c.expect_locked,
                "case {:?}: expected locked={}, got {}", c, c.expect_locked, locked
            );
        }
    }

    #[test]
    fn pending_on_published_gets_pending_actions() {
        // Full path: a pending Evidence item on a PUBLISHED document must
        // offer approve/reject/edit. This is the fix for Roman's 57 stuck
        // ComplaintAllegations.
        let locked = is_post_ingest("PUBLISHED") && is_post_ingest_locked_status("pending");
        assert!(!locked);
        let actions = compute_available_actions(&EntityCategory::Evidence, "pending", locked);
        assert!(actions.contains(&"approve".to_string()));
        assert!(actions.contains(&"reject".to_string()));
        assert!(actions.contains(&"edit".to_string()));
    }

    #[test]
    fn approved_on_published_gets_no_actions() {
        // Regression guard: already-ingested items stay read-only.
        let locked = is_post_ingest("PUBLISHED") && is_post_ingest_locked_status("approved");
        assert!(locked);
        let actions = compute_available_actions(&EntityCategory::Evidence, "approved", locked);
        assert!(actions.is_empty());
    }
}
