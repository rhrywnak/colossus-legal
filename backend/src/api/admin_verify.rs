//! Admin endpoint: verify or reject an evidence item.
//!
//! Inserts (or updates) a row in `audit_verifications` and logs the
//! action to the audit trail. The verification status is displayed
//! in the Document Workspace evidence cards.

use axum::{extract::Path, extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::state::AppState;

// ── Request / Response DTOs ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    /// One of: "verified", "rejected", "pending"
    pub status: String,
    /// Optional reviewer notes
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub evidence_id: String,
    pub status: String,
    pub verified_by: String,
}

// ── Handler ──────────────────────────────────────────────────────

/// POST /admin/documents/:id/evidence/:eid/verify
///
/// Sets the verification status for a specific evidence item.
/// Uses INSERT ON CONFLICT to upsert — if a verification already exists
/// for this (document_id, evidence_id), it updates the existing row.
///
/// ## Rust Learning: Path Extractors with Tuples
///
/// Axum can destructure multi-segment path parameters into a tuple:
/// `Path((doc_id, evidence_id))` matches `/documents/:id/evidence/:eid/verify`.
/// The order must match the order of path segments.
pub async fn verify_evidence(
    user: AuthUser,
    State(state): State<AppState>,
    Path((doc_id, evidence_id)): Path<(String, String)>,
    Json(body): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, AppError> {
    require_admin(&user)?;

    // Validate status value
    let valid_statuses = ["verified", "rejected", "pending"];
    if !valid_statuses.contains(&body.status.as_str()) {
        return Err(AppError::BadRequest {
            message: format!(
                "Invalid status '{}'. Must be one of: {}",
                body.status,
                valid_statuses.join(", ")
            ),
            details: serde_json::json!({"field": "status"}),
        });
    }

    tracing::info!(
        user = %user.username,
        doc_id = %doc_id,
        evidence_id = %evidence_id,
        status = %body.status,
        "Verify evidence"
    );

    // Upsert verification row.
    // If a row already exists for this (document_id, evidence_id) pair,
    // update it. Otherwise insert a new row.
    //
    // NOTE: audit_verifications doesn't have a unique constraint on
    // (document_id, evidence_id), so we delete any existing rows first
    // and insert fresh. This keeps the most recent verification only.
    sqlx::query(
        "DELETE FROM audit_verifications
         WHERE document_id = $1 AND evidence_id = $2",
    )
    .bind(&doc_id)
    .bind(&evidence_id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to clear old verification: {e}"),
    })?;

    // "pending" means undo — just delete the row (done above), don't insert.
    if body.status != "pending" {
        sqlx::query(
            "INSERT INTO audit_verifications
             (document_id, evidence_id, verified_by, status, notes)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&doc_id)
        .bind(&evidence_id)
        .bind(&user.username)
        .bind(&body.status)
        .bind(&body.notes)
        .execute(&state.pg_pool)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to insert verification: {e}"),
        })?;
    }

    // Log to audit trail
    log_admin_action(
        &state.audit_repo,
        &user.username,
        "evidence.verify",
        Some("evidence"),
        Some(&evidence_id),
        Some(serde_json::json!({
            "document_id": doc_id,
            "status": body.status,
            "notes": body.notes,
        })),
    )
    .await;

    Ok(Json(VerifyResponse {
        evidence_id,
        status: body.status,
        verified_by: user.username,
    }))
}
