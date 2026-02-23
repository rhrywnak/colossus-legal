//! Semantic search endpoint.
//!
//! `POST /search` — embeds the user's query text using fastembed (with the
//! `search_query:` prefix required by nomic-embed-text), then searches
//! Qdrant for the closest vectors and returns ranked results.
//!
//! ## Pattern: Axum JSON request extraction
//! `Json(req): Json<SearchRequest>` tells Axum to automatically:
//! 1. Read the POST body
//! 2. Deserialize it as JSON into `SearchRequest` via serde
//! 3. Return 400 Bad Request if deserialization fails
//!
//! This is the Axum equivalent of Express's `req.body` with validation built in.

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::api::embed::ErrorResponse;
use crate::services::embedding_service::EmbeddingService;
use crate::services::graph_expander;
use crate::services::qdrant_service;
use crate::state::AppState;

/// Request body for semantic search.
///
/// ## Pattern: Optional struct fields with serde
/// `Option<usize>` means the field can be omitted from the JSON body.
/// When missing, serde sets it to `None` — no error, no default needed
/// at the struct level. The handler applies defaults in its logic.
#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    pub node_types: Option<Vec<String>>,
    pub expand: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchHit>,
    pub total: usize,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_nodes: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub node_id: String,
    pub node_type: String,
    pub title: String,
    pub score: f32,
    pub document_id: Option<String>,
    pub page_number: Option<String>,
}

/// POST /search
///
/// Embeds the query via fastembed (spawn_blocking), searches Qdrant, returns hits.
pub async fn semantic_search(
    State(state): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let start = Instant::now();

    // Validate: query must not be empty
    let query = req.query.trim().to_string();
    if query.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "query must not be empty".to_string(),
            }),
        ));
    }

    // Default limit to 10, cap at 50
    let limit = req.limit.unwrap_or(10).min(50);

    // Embed the query using "search_query:" prefix (nomic convention)
    let query_text = format!("search_query: {query}");
    let cache_path = state.config.fastembed_cache_path.clone();

    let vector = tokio::task::spawn_blocking(move || {
        let mut service = EmbeddingService::new(&cache_path)?;
        service.embed_one(&query_text)
    })
    .await
    .map_err(|e| {
        tracing::error!("spawn_blocking panicked: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "embedding task failed".to_string(),
            }),
        )
    })?
    .map_err(|e| {
        tracing::error!("Embedding failed: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("embedding error: {e}"),
            }),
        )
    })?;

    // Search Qdrant
    let http_client = reqwest::Client::new();
    let results = qdrant_service::search_points(
        &http_client,
        &state.config.qdrant_url,
        vector,
        limit,
        req.node_types,
    )
    .await
    .map_err(|e| {
        tracing::error!("Qdrant search failed: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("search error: {e}"),
            }),
        )
    })?;

    // Map to response
    let hits: Vec<SearchHit> = results
        .into_iter()
        .map(|r| SearchHit {
            node_id: r.node_id,
            node_type: r.node_type,
            title: r.title,
            score: r.score,
            document_id: r.document_id,
            page_number: r.page_number,
        })
        .collect();

    let total = hits.len();

    // Graph expansion (H.3): when expand=true, expand top hits through Neo4j
    let (context, context_nodes) = if req.expand.unwrap_or(false) && !hits.is_empty() {
        let seed_ids: Vec<(String, String)> = hits
            .iter()
            .map(|h| (h.node_id.clone(), h.node_type.clone()))
            .collect();

        tracing::info!(
            seed_count = seed_ids.len(),
            seeds = ?seed_ids,
            "Graph expansion: passing seed IDs to expand_context"
        );

        match graph_expander::expand_context(&state.graph, seed_ids, 6000).await {
            Ok(expanded) => (Some(expanded.formatted_text), Some(expanded.unique_nodes)),
            Err(e) => {
                tracing::warn!("Graph expansion failed (non-fatal): {e}");
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(Json(SearchResponse {
        query,
        results: hits,
        total,
        duration_ms,
        context,
        context_nodes,
    }))
}
