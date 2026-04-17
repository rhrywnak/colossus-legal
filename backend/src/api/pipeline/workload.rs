//! Reviewer workload endpoint — aggregates review progress per reviewer.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct ReviewerWorkload {
    pub username: String,
    pub display_name: Option<String>,
    pub assigned_documents: i64,
    pub reviewed_documents: i64,
    pub pending_documents: i64,
    pub total_items: i64,
    pub approved_items: i64,
    pub pending_items: i64,
    pub rejected_items: i64,
}

#[derive(Debug, Serialize)]
pub struct WorkloadResponse {
    pub reviewers: Vec<ReviewerWorkload>,
    pub unassigned_documents: i64,
}

/// Raw row from the workload query — avoids a complex tuple type.
#[derive(sqlx::FromRow)]
struct WorkloadRow {
    username: String,
    display_name: Option<String>,
    assigned_documents: i64,
    reviewed_documents: i64,
    total_items: i64,
    approved_items: i64,
    pending_items: i64,
    rejected_items: i64,
}

/// GET /reviewers/workload — returns per-reviewer workload summary.
pub async fn workload_handler(
    _user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<WorkloadResponse>, AppError> {
    let pool = &state.pipeline_pool;

    // Per-reviewer document and item counts.
    // A document is "reviewed" when ALL its extraction_items have review_status != 'pending'
    // and at least one extraction_item exists.
    let rows: Vec<WorkloadRow> = sqlx::query_as(
        r#"
        SELECT
            d.assigned_reviewer AS username,
            ku.display_name,
            COUNT(DISTINCT d.id) AS assigned_documents,
            COUNT(DISTINCT d.id) FILTER (
                WHERE NOT EXISTS (
                    SELECT 1 FROM extraction_items ei2
                    WHERE ei2.document_id = d.id AND ei2.review_status = 'pending'
                )
                AND EXISTS (
                    SELECT 1 FROM extraction_items ei3
                    WHERE ei3.document_id = d.id
                )
            ) AS reviewed_documents,
            COALESCE(SUM(item_counts.total), 0) AS total_items,
            COALESCE(SUM(item_counts.approved), 0) AS approved_items,
            COALESCE(SUM(item_counts.pending), 0) AS pending_items,
            COALESCE(SUM(item_counts.rejected), 0) AS rejected_items
        FROM documents d
        LEFT JOIN known_users ku ON ku.username = d.assigned_reviewer
        LEFT JOIN LATERAL (
            SELECT
                COUNT(*) AS total,
                COUNT(*) FILTER (WHERE review_status = 'approved') AS approved,
                COUNT(*) FILTER (WHERE review_status = 'pending') AS pending,
                COUNT(*) FILTER (WHERE review_status = 'rejected') AS rejected
            FROM extraction_items ei
            WHERE ei.document_id = d.id
        ) item_counts ON true
        WHERE d.assigned_reviewer IS NOT NULL
        GROUP BY d.assigned_reviewer, ku.display_name
        ORDER BY d.assigned_reviewer
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to query workload: {e}"),
    })?;

    let reviewers: Vec<ReviewerWorkload> = rows
        .into_iter()
        .map(|r| ReviewerWorkload {
            pending_documents: r.assigned_documents - r.reviewed_documents,
            username: r.username,
            display_name: r.display_name,
            assigned_documents: r.assigned_documents,
            reviewed_documents: r.reviewed_documents,
            total_items: r.total_items,
            approved_items: r.approved_items,
            pending_items: r.pending_items,
            rejected_items: r.rejected_items,
        })
        .collect();

    // Count unassigned documents
    let unassigned: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM documents WHERE assigned_reviewer IS NULL")
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to count unassigned: {e}"),
            })?;

    Ok(Json(WorkloadResponse {
        reviewers,
        unassigned_documents: unassigned.0,
    }))
}
