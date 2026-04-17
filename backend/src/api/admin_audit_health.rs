//! GET /api/admin/audit/health — Automated data quality checks.
//!
//! ## Rust Learning — Concurrent Futures with tokio::join!
//!
//! This endpoint runs multiple independent database queries. Rather than
//! running them sequentially (query1.await; query2.await; ...), we use
//! `tokio::join!` to run them concurrently. This means all queries
//! execute in parallel, and the total time is the slowest single query
//! rather than the sum of all queries. This is safe because each query
//! uses its own connection from the pool.

use axum::{extract::State, Json};
use chrono::Utc;
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::services::audit_checks::{self, AuditCheck};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct AuditHealthResponse {
    pub checked_at: String,
    pub summary: AuditSummary,
    pub checks: Vec<AuditCheck>,
}

#[derive(Debug, Serialize)]
pub struct AuditSummary {
    pub total_documents: usize,
    pub total_evidence: usize,
    pub total_qdrant_points: usize,
    pub completeness_pct: f64,
    pub issues_found: usize,
}

/// GET /api/admin/audit/health — Run all data quality checks.
///
/// Applies a 10-second timeout to the entire check suite. Returns
/// structured results with pass/warn/fail status per check.
pub async fn audit_health(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<AuditHealthResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, "GET /api/admin/audit/health");

    let result =
        tokio::time::timeout(std::time::Duration::from_secs(10), run_all_checks(&state)).await;

    match result {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(AppError::Internal {
            message: "Audit health checks timed out after 10 seconds".into(),
        }),
    }
}

async fn run_all_checks(state: &AppState) -> AuditHealthResponse {
    let storage_path = &state.config.document_storage_path;

    // Run all four checks concurrently
    let (
        pdf_check,
        (evidence_check, total_evidence, complete_evidence),
        (qdrant_check, qdrant_points),
        orphan_check,
    ) = tokio::join!(
        audit_checks::check_pdf_match(&state.graph, storage_path),
        audit_checks::check_evidence_completeness(&state.graph),
        audit_checks::check_qdrant_reconciliation(
            &state.graph,
            &state.http_client,
            &state.config.qdrant_url,
        ),
        audit_checks::check_orphaned_nodes(&state.graph),
    );

    // Count total documents from pdf_check details + passes
    let doc_issues = pdf_check.details.len();
    let total_documents = pdf_check
        .message
        .split_whitespace()
        .next()
        .and_then(|n| n.parse::<usize>().ok())
        .unwrap_or(0);

    let completeness_pct = if total_evidence > 0 {
        (complete_evidence as f64 / total_evidence as f64) * 100.0
    } else {
        100.0
    };

    let checks = vec![pdf_check, evidence_check, qdrant_check, orphan_check];
    let issues_found: usize = checks.iter().map(|c| c.details.len()).sum();

    AuditHealthResponse {
        checked_at: Utc::now().to_rfc3339(),
        summary: AuditSummary {
            total_documents: total_documents.max(doc_issues),
            total_evidence,
            total_qdrant_points: qdrant_points,
            completeness_pct,
            issues_found,
        },
        checks,
    }
}
