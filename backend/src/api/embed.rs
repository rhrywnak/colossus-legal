//! Admin endpoint for running the embedding pipeline.
//!
//! `POST /admin/embed-all` — reads all nodes from Neo4j, generates
//! embeddings via fastembed, and upserts them to Qdrant.
//! This is a long-running operation (~30-120 seconds) intended for
//! admin use only.

use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use std::collections::HashMap;

use crate::auth::{require_admin, AuthUser};
use crate::services::embedding_pipeline;
use crate::state::AppState;

/// Response DTO for the embedding pipeline result.
#[derive(Debug, Serialize)]
pub struct EmbeddingResultDto {
    pub total_nodes: usize,
    pub embedded_count: usize,
    pub skipped: usize,
    pub nodes_by_type: HashMap<String, usize>,
    pub duration_seconds: f64,
    pub errors: Vec<String>,
}

/// Error response body.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// POST /admin/embed-all
///
/// Runs the full embedding pipeline: Neo4j → fastembed → Qdrant.
pub async fn run_embed_all(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<EmbeddingResultDto>, (StatusCode, Json<ErrorResponse>)> {
    require_admin(&user).map_err(|e| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: e.message }),
        )
    })?;
    tracing::info!("{} POST /admin/embed-all", user.username);
    let http_client = &state.http_client;

    // The HTTP endpoint always does a full (non-incremental) embed.
    // Incremental mode is available only via the CLI.
    let result = embedding_pipeline::run_embedding_pipeline(
        &state.graph,
        http_client,
        &state.config.qdrant_url,
        &state.config.fastembed_cache_path,
        false, // incremental
        false, // dry_run
        // FIXME(P2-Nx-C): replace literal 768 with state.embedding_provider.dimensions()
        // once P2-Nx-B has added the provider to AppState.
        768,
    )
    .await
    .map_err(|e| {
        tracing::error!("Embedding pipeline failed: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(EmbeddingResultDto {
        total_nodes: result.total_nodes,
        embedded_count: result.embedded_count,
        skipped: result.skipped,
        nodes_by_type: result.nodes_by_type,
        duration_seconds: result.duration_seconds,
        errors: result.errors,
    }))
}
