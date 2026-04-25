//! `GET /api/admin/pipeline/documents/:id/completeness` — entity-level
//! completeness verification.
//!
//! Answers one question for a given document: *did everything we
//! extracted make it into the graph and the vector store?* The check is
//! entity-level, not count-level: for each approved extraction item we
//! compute the expected Neo4j id, batch-verify existence, then
//! batch-verify a Qdrant point for every found node.
//!
//! Design: `COMPLETENESS_VERIFICATION_REDESIGN_v1.md`. The previous
//! count-based comparison is deleted — `MERGE` deduplication of shared
//! parties made count equality unreachable for any document that
//! shared an entity with another.

use axum::{extract::Path, extract::State, Json};
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::models::document_status::{STATUS_INDEXED, STATUS_PUBLISHED};
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository::{self, steps};
use crate::state::AppState;

use super::completeness_helpers::{
    compute_expected_neo4j_ids, document_node_exists, verify_neo4j_nodes, verify_qdrant_points,
};

// ─────────────────────────────────────────────────────────────────────
// Response DTOs
// ─────────────────────────────────────────────────────────────────────

/// Response body for the completeness endpoint.
///
/// Entity-level fields (nodes_missing / points_missing) are the new
/// canonical source of truth. `checks: Vec<CompletenessCheck>` is
/// preserved in a synthetic form so any external admin tooling that
/// iterates the old array still sees a structured pass/fail per
/// verification category.
#[derive(Debug, Serialize)]
pub struct CompletenessResponse {
    pub document_id: String,
    /// Overall result: `"pass"`, `"warn"`, or `"fail"`.
    pub status: String,
    /// Total approved items fed into the verification.
    pub total_items: usize,
    /// Items whose expected Neo4j node was found.
    pub nodes_verified: usize,
    /// Expected Neo4j ids that are missing from the graph.
    pub nodes_missing: Vec<String>,
    /// Found Neo4j nodes that have a Qdrant point.
    pub points_verified: usize,
    /// Found Neo4j node ids that have no Qdrant point.
    pub points_missing: Vec<String>,
    /// Whether the Document node exists in Neo4j.
    pub document_node: bool,
    /// Legacy-shape per-check rollup. Back-compat with admin tooling
    /// that iterated the old `checks` array. Each entry here corresponds
    /// to one of the entity-level verification categories.
    pub checks: Vec<CompletenessCheck>,
    /// Whether this run transitioned the document to `PUBLISHED`.
    pub published: bool,
}

/// One row of the legacy `checks` array. `expected` / `actual` are kept
/// as loose counts (e.g., total_items vs verified) so external consumers
/// have numeric fields to render; the authoritative per-id detail lives
/// in [`CompletenessResponse::nodes_missing`] / `points_missing`.
#[derive(Debug, Serialize)]
pub struct CompletenessCheck {
    pub name: String,
    pub status: String,
    pub expected: usize,
    pub actual: usize,
    pub message: String,
}

// ─────────────────────────────────────────────────────────────────────
// Handler
// ─────────────────────────────────────────────────────────────────────

/// Core completeness logic — callable from the HTTP handler AND the
/// automated pipeline.
///
/// ## Step-failure recording (preserved from Batch 5)
///
/// Wrapped in an outer fn that records step start; on Ok it records
/// complete, on any propagated Err it records failure so
/// `pipeline_steps.status` flips from `running` → `failed` rather than
/// stranding the row. Mirrors the pattern in `pipeline/steps/completeness.rs`.
pub(crate) async fn run_completeness(
    state: &AppState,
    doc_id: &str,
    username: &str,
) -> Result<CompletenessResponse, AppError> {
    let start = std::time::Instant::now();

    let step_id = steps::record_step_start(
        &state.pipeline_pool,
        doc_id,
        "completeness",
        username,
        &serde_json::json!({}),
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Step logging: {e}"),
    })?;

    let result = run_completeness_impl(state, doc_id, username).await;
    let duration_secs = start.elapsed().as_secs_f64();

    match result {
        Ok(response) => {
            steps::record_step_complete(
                &state.pipeline_pool,
                step_id,
                duration_secs,
                &serde_json::json!({
                    "total_items": response.total_items,
                    "nodes_verified": response.nodes_verified,
                    "nodes_missing": response.nodes_missing.len(),
                    "points_verified": response.points_verified,
                    "points_missing": response.points_missing.len(),
                    "document_node": response.document_node,
                    "status": response.status,
                    "published": response.published,
                }),
            )
            .await
            .ok();
            Ok(response)
        }
        Err(e) => {
            let err_msg = match &e {
                AppError::BadRequest { message, .. } => message.clone(),
                AppError::NotFound { message } => message.clone(),
                AppError::Unauthorized { message } => message.clone(),
                AppError::Forbidden { message } => message.clone(),
                AppError::Conflict { message, .. } => message.clone(),
                AppError::Internal { message } => message.clone(),
            };
            if let Err(rec_err) = steps::record_step_failure(
                &state.pipeline_pool,
                step_id,
                duration_secs,
                &err_msg,
            )
            .await
            {
                tracing::warn!(
                    doc_id = %doc_id, step_id, error = %rec_err,
                    "Completeness: record_step_failure failed (non-fatal)"
                );
            }
            Err(e)
        }
    }
}

/// Core body — builds the CompletenessResponse. Does not touch
/// `pipeline_steps`; the outer [`run_completeness`] wraps this and
/// records completion/failure.
async fn run_completeness_impl(
    state: &AppState,
    doc_id: &str,
    username: &str,
) -> Result<CompletenessResponse, AppError> {
    // 1. Document exists in Postgres.
    let document = pipeline_repository::get_document(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    // 2. Latest completed extraction run.
    let run_id = pipeline_repository::get_latest_completed_run(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("No completed extraction run for document '{doc_id}'"),
        })?;

    // 3. Approved extraction items for that run.
    let items =
        pipeline_repository::get_approved_items_for_document(&state.pipeline_pool, doc_id, run_id)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("DB error: {e}"),
            })?;
    let total_items = items.len();

    // 4. Compute the expected Neo4j id per approved item.
    let expected: Vec<(i32, String)> = compute_expected_neo4j_ids(&items, doc_id);
    let expected_ids: Vec<String> = expected.iter().map(|(_, id)| id.clone()).collect();

    // 5. Document node existence — FAIL condition if missing.
    let document_node = document_node_exists(&state.graph, doc_id).await?;

    // 6. Batch Neo4j verification.
    let nodes_missing = verify_neo4j_nodes(&state.graph, &expected_ids).await?;
    let missing_set: std::collections::HashSet<&String> = nodes_missing.iter().collect();
    let found_node_ids: Vec<String> = expected_ids
        .iter()
        .filter(|id| !missing_set.contains(id))
        .cloned()
        .collect();
    let nodes_verified = found_node_ids.len();

    // 7. Batch Qdrant verification — only for nodes we actually found.
    let points_missing = verify_qdrant_points(
        &state.http_client,
        &state.config.qdrant_url,
        &found_node_ids,
    )
    .await?;
    let points_verified = found_node_ids.len() - points_missing.len();

    // 8. Pass/warn/fail.
    let status = if !document_node || !nodes_missing.is_empty() {
        "fail"
    } else if !points_missing.is_empty() {
        "warn"
    } else {
        "pass"
    };

    // 9. Synthetic per-check rollup for back-compat with the old
    //    `checks` array shape. Values chosen so `expected`/`actual`
    //    stay meaningful: total vs verified for nodes, verified vs
    //    missing-count for points, 0 vs 1 for the Document node.
    let checks = vec![
        CompletenessCheck {
            name: "neo4j_document_node".to_string(),
            status: if document_node { "pass" } else { "fail" }.to_string(),
            expected: 1,
            actual: if document_node { 1 } else { 0 },
            message: if document_node {
                format!("Document node present for '{doc_id}'")
            } else {
                format!("Document node missing for '{doc_id}'")
            },
        },
        CompletenessCheck {
            name: "neo4j_node_exists".to_string(),
            status: if nodes_missing.is_empty() { "pass" } else { "fail" }.to_string(),
            expected: expected_ids.len(),
            actual: nodes_verified,
            message: format!(
                "{} of {} expected entity nodes present in Neo4j ({} missing)",
                nodes_verified,
                expected_ids.len(),
                nodes_missing.len()
            ),
        },
        CompletenessCheck {
            name: "qdrant_point_exists".to_string(),
            status: if points_missing.is_empty() { "pass" } else { "warn" }.to_string(),
            expected: found_node_ids.len(),
            actual: points_verified,
            message: format!(
                "{} of {} Neo4j nodes have a Qdrant point ({} missing)",
                points_verified,
                found_node_ids.len(),
                points_missing.len()
            ),
        },
    ];

    // 10. Transition to PUBLISHED on a pass (or warn — warnings are
    //     non-blocking). Only when not already PUBLISHED to avoid an
    //     unnecessary write.
    let published = if status != "fail" && document.status != STATUS_PUBLISHED {
        pipeline_repository::update_document_status(&state.pipeline_pool, doc_id, STATUS_PUBLISHED)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to update status: {e}"),
            })?;
        tracing::info!(
            doc_id = %doc_id, status,
            "Completeness {status} — status → {STATUS_PUBLISHED}"
        );
        true
    } else {
        false
    };

    log_admin_action(
        &state.audit_repo,
        username,
        "pipeline.document.completeness",
        Some("document"),
        Some(doc_id),
        Some(serde_json::json!({
            "status": status,
            "published": published,
            "total_items": total_items,
            "nodes_missing": nodes_missing.len(),
            "points_missing": points_missing.len(),
        })),
    )
    .await;

    Ok(CompletenessResponse {
        document_id: doc_id.to_string(),
        status: status.to_string(),
        total_items,
        nodes_verified,
        nodes_missing,
        points_verified,
        points_missing,
        document_node,
        checks,
        published,
    })
}

/// `GET /api/admin/pipeline/documents/:id/completeness`
///
/// HTTP handler — thin wrapper around `run_completeness`. Auth and
/// status guard, then delegate.
pub async fn completeness_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<CompletenessResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "GET completeness");

    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    if document.status != STATUS_INDEXED && document.status != STATUS_PUBLISHED {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot check completeness: status is '{}', expected '{STATUS_INDEXED}'",
                document.status
            ),
            details: serde_json::json!({ "status": document.status }),
        });
    }

    let result = run_completeness(&state, &doc_id, &user.username).await?;
    Ok(Json(result))
}
