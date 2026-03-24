//! Admin endpoint: flag an evidence item with an issue.
//!
//! Inserts a row in `audit_findings` and logs the action to the
//! audit trail. Flags are displayed in the Document Workspace
//! evidence cards as warning indicators.

use axum::{extract::Path, extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::state::AppState;

// ── Request / Response DTOs ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct FlagRequest {
    /// One of: "low", "medium", "high", "critical"
    pub severity: String,
    /// Description of the issue
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct FlagResponse {
    pub id: i64,
    pub evidence_id: String,
    pub severity: String,
    pub flagged_by: String,
}

// ── Handler ──────────────────────────────────────────────────────

/// POST /admin/documents/:id/evidence/:eid/flag
///
/// Creates a new audit finding (flag) for a specific evidence item.
/// Multiple flags can exist per evidence — this always inserts a new row.
pub async fn flag_evidence(
    user: AuthUser,
    State(state): State<AppState>,
    Path((doc_id, evidence_id)): Path<(String, String)>,
    Json(body): Json<FlagRequest>,
) -> Result<Json<FlagResponse>, AppError> {
    require_admin(&user)?;

    // Validate severity
    let valid_severities = ["low", "medium", "high", "critical"];
    if !valid_severities.contains(&body.severity.as_str()) {
        return Err(AppError::BadRequest {
            message: format!(
                "Invalid severity '{}'. Must be one of: {}",
                body.severity,
                valid_severities.join(", ")
            ),
            details: serde_json::json!({"field": "severity"}),
        });
    }

    tracing::info!(
        user = %user.username,
        doc_id = %doc_id,
        evidence_id = %evidence_id,
        severity = %body.severity,
        "Flag evidence"
    );

    // Insert finding row
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO audit_findings
         (document_id, evidence_id, finding_type, severity, description, found_by)
         VALUES ($1, $2, 'manual_flag', $3, $4, $5)
         RETURNING id",
    )
    .bind(&doc_id)
    .bind(&evidence_id)
    .bind(&body.severity)
    .bind(&body.description)
    .bind(&user.username)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to insert finding: {e}"),
    })?;

    // Log to audit trail
    log_admin_action(
        &state.audit_repo,
        &user.username,
        "evidence.flag",
        Some("evidence"),
        Some(&evidence_id),
        Some(serde_json::json!({
            "document_id": doc_id,
            "severity": body.severity,
            "description": body.description,
        })),
    )
    .await;

    Ok(Json(FlagResponse {
        id: row.0,
        evidence_id,
        severity: body.severity,
        flagged_by: user.username,
    }))
}
