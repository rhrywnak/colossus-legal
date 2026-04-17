//! Admin endpoint to trigger Qdrant reindexing.
//!
//! This is the modern replacement for `POST /admin/embed-all`, supporting
//! incremental and full modes. The old endpoint is kept for Ansible backward
//! compatibility.
//!
//! ## Rust Learning: Long-Running Handlers
//!
//! Embedding 200+ nodes takes 30-120 seconds. Axum handlers are async, so
//! the server continues handling other requests while this one awaits the
//! pipeline. If the HTTP client times out, the response is lost but the
//! pipeline still completes server-side.

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::services::{embedding_pipeline, qdrant_service};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ReindexRequest {
    /// "incremental" (default) or "full"
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_mode() -> String {
    "incremental".to_string()
}

#[derive(Debug, Serialize)]
pub struct ReindexResponse {
    pub mode: String,
    pub new_points: usize,
    pub skipped: usize,
    pub total: usize,
    pub duration_ms: u64,
}

/// POST /api/admin/reindex — Trigger Qdrant reindexing.
pub async fn trigger_reindex(
    user: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ReindexRequest>,
) -> Result<(StatusCode, Json<ReindexResponse>), AppError> {
    require_admin(&user)?;

    let incremental = req.mode != "full";
    let clean = req.mode == "full";

    tracing::info!(user = %user.username, mode = %req.mode, "Admin triggered reindex");

    // Full mode: delete collection first, then pipeline recreates it
    if clean {
        qdrant_service::delete_collection(&state.http_client, &state.config.qdrant_url)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to delete collection: {e}"),
            })?;
        tracing::info!("Full reindex: Qdrant collection deleted");
    }

    let result = embedding_pipeline::run_embedding_pipeline(
        &state.graph,
        &state.http_client,
        &state.config.qdrant_url,
        &state.config.fastembed_cache_path,
        incremental,
        false, // dry_run
        state.embedding_provider.dimensions(),
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Reindex failed: {e}"),
    })?;

    let duration_ms = (result.duration_seconds * 1000.0) as u64;
    let total = result.embedded_count + result.skipped;

    tracing::info!(
        user = %user.username,
        mode = %req.mode,
        new = result.embedded_count,
        skipped = result.skipped,
        duration_ms,
        "Reindex complete"
    );

    log_admin_action(
        &state.audit_repo,
        &user.username,
        "reindex.trigger",
        Some("index"),
        None,
        Some(serde_json::json!({ "mode": &req.mode })),
    )
    .await;

    Ok((
        StatusCode::OK,
        Json(ReindexResponse {
            mode: req.mode,
            new_points: result.embedded_count,
            skipped: result.skipped,
            total,
            duration_ms,
        }),
    ))
}
