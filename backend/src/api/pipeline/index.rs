//! POST /api/admin/pipeline/documents/:id/index — Vector Indexer.
//!
//! Reads nodes for a specific document from Neo4j, generates embeddings
//! via fastembed, and upserts vectors to the `colossus_evidence` Qdrant
//! collection. This is the final step of the pipeline: after ingest
//! writes nodes to Neo4j, index makes them searchable via semantic search.
//!
//! ## Rust Learning: spawn_blocking for CPU-bound work
//!
//! fastembed runs ONNX inference (matrix multiplication, attention layers)
//! which is CPU-bound and synchronous. Calling it directly inside an async
//! function would block the tokio runtime, starving other tasks (HTTP
//! requests, DB queries). `tokio::task::spawn_blocking` moves the work
//! to a dedicated thread pool for blocking operations.
//!
//! The `move` closure captures `texts` and `cache_path` by value —
//! ownership transfers into the blocking thread. This is required because
//! the closure outlives the current async scope.
//!
//! ## Rust Learning: HashMap entry API for counting
//!
//! `*map.entry(key).or_insert(0) += 1` is the idiomatic way to count
//! occurrences. `entry()` returns an `Entry` enum — either `Occupied`
//! (key exists) or `Vacant` (key missing). `or_insert(0)` inserts 0 for
//! new keys and returns a `&mut usize` either way, which we dereference
//! and increment.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::Instant;

use axum::{extract::Path, extract::State, Json};
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::embedding_repository;
use crate::repositories::pipeline_repository::{self, steps};
use crate::services::embedding_service::{EmbeddingError, EmbeddingService};
use crate::services::embedding_text::build_embedding_text;
use crate::services::qdrant_service::{self, QdrantPoint};
use crate::state::AppState;

// ── Response DTO ────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct IndexResponse {
    pub document_id: String,
    pub status: String,
    pub nodes_embedded: usize,
    pub nodes_by_type: HashMap<String, usize>,
    pub qdrant_collection: String,
    pub duration_secs: f64,
    pub errors: Vec<String>,
}

// ── Handler ─────────────────────────────────────────────────────

/// Core logic for vector indexing — callable from handler AND process endpoint.
///
/// Embeds all Neo4j nodes for a document and upserts to Qdrant.
/// Does NOT check document status — caller is responsible for validation.
pub(crate) async fn run_index(
    state: &AppState,
    doc_id: &str,
    username: &str,
) -> Result<IndexResponse, AppError> {
    let start = Instant::now();

    let step_id = steps::record_step_start(
        &state.pipeline_pool, doc_id, "index", username, &serde_json::json!({}),
    ).await.map_err(|e| AppError::Internal { message: format!("Step logging: {e}") })?;

    // 1. Fetch document — must exist (for title/metadata, not status check)
    let _document = pipeline_repository::get_document(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    // 2. Query Neo4j for all nodes belonging to this document
    let nodes = embedding_repository::fetch_nodes_for_document(&state.graph, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j fetch error: {e}"),
        })?;

    tracing::info!(doc_id = %doc_id, node_count = nodes.len(), "Fetched nodes from Neo4j");

    if nodes.is_empty() {
        return Err(AppError::NotFound {
            message: format!("No nodes found in Neo4j for document '{doc_id}'"),
        });
    }

    // 4. Build embedding text for each node
    let texts: Vec<String> = nodes
        .iter()
        .map(|n| build_embedding_text(&n.node_type, &n.properties))
        .collect();

    // Count nodes by type (for the response)
    let mut nodes_by_type: HashMap<String, usize> = HashMap::new();
    for node in &nodes {
        *nodes_by_type.entry(node.node_type.clone()).or_insert(0) += 1;
    }

    // 5. Generate embeddings via fastembed (in spawn_blocking)
    tracing::info!("Embedding {} texts...", texts.len());
    let cache_path = state.config.fastembed_cache_path.clone();
    let vectors = tokio::task::spawn_blocking(move || {
        let mut service = EmbeddingService::new(&cache_path)?;
        let mut all_vectors = Vec::new();
        for chunk in texts.chunks(50) {
            let batch = chunk.to_vec();
            let embeddings = service.embed_batch(batch)?;
            all_vectors.extend(embeddings);
        }
        Ok::<Vec<Vec<f32>>, EmbeddingError>(all_vectors)
    })
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Embedding task panicked: {e}"),
    })?
    .map_err(|e| AppError::Internal {
        message: format!("Embedding error: {e}"),
    })?;

    // 6. Build Qdrant points with payload metadata
    let mut points = Vec::new();
    let mut errors = Vec::new();

    for (i, node) in nodes.iter().enumerate() {
        let Some(vector) = vectors.get(i) else {
            errors.push(format!("Missing vector for node {}", node.id));
            continue;
        };

        let title = node
            .properties
            .get("title")
            .or_else(|| node.properties.get("name"))
            .cloned()
            .unwrap_or_default();

        let page_number = node.properties.get("page_number").cloned();

        let mut payload = serde_json::json!({
            "node_id": node.id,
            "node_type": node.node_type,
            "title": title,
            "document_id": doc_id,
            "source_document": doc_id,
        });

        // Add page_number if present
        if let Some(ref page) = page_number {
            if let Some(obj) = payload.as_object_mut() {
                obj.insert(
                    "page_number".to_string(),
                    serde_json::Value::String(page.clone()),
                );
            }
        }

        points.push(QdrantPoint {
            id: node_id_to_point_id(&node.id),
            vector: vector.clone(),
            payload,
        });
    }

    let embedded_count = points.len();

    // 7. Ensure Qdrant collection exists
    qdrant_service::ensure_collection(&state.http_client, &state.config.qdrant_url)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Qdrant collection error: {e}"),
        })?;

    // 8. Upsert points to Qdrant
    tracing::info!("Upserting {} points to Qdrant...", embedded_count);
    qdrant_service::upsert_points(&state.http_client, &state.config.qdrant_url, points)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Qdrant upsert error: {e}"),
        })?;

    // 9. Update pipeline document status → INDEXED
    pipeline_repository::update_document_status(&state.pipeline_pool, doc_id, "INDEXED")
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update document status: {e}"),
        })?;

    let duration = start.elapsed().as_secs_f64();
    tracing::info!(
        doc_id = %doc_id, embedded_count, duration_secs = format!("{duration:.2}"),
        "Index complete"
    );

    log_admin_action(
        &state.audit_repo,
        username,
        "pipeline.document.index",
        Some("document"),
        Some(doc_id),
        Some(serde_json::json!({
            "nodes_embedded": embedded_count,
            "qdrant_collection": "colossus_evidence",
        })),
    )
    .await;

    steps::record_step_complete(
        &state.pipeline_pool, step_id, duration,
        &serde_json::json!({"nodes_embedded": embedded_count, "collection": "colossus_evidence", "errors": &errors}),
    ).await.ok();

    // 10. Return summary
    Ok(IndexResponse {
        document_id: doc_id.to_string(),
        status: "INDEXED".to_string(),
        nodes_embedded: embedded_count,
        nodes_by_type,
        qdrant_collection: "colossus_evidence".to_string(),
        duration_secs: duration,
        errors,
    })
}

/// POST /api/admin/pipeline/documents/:id/index
///
/// HTTP handler — thin wrapper around `run_index`.
/// Checks admin auth and status guard, then delegates to core logic.
pub async fn index_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<IndexResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST index");

    // Status guard
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    if document.status != "INGESTED" {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot index: status is '{}', expected 'INGESTED'",
                document.status
            ),
            details: serde_json::json!({ "status": document.status }),
        });
    }

    let result = run_index(&state, &doc_id, &user.username).await?;
    Ok(Json(result))
}

/// Convert a node ID string to a deterministic u64 for Qdrant point IDs.
/// Same hashing approach as `embedding_pipeline::node_id_to_point_id`.
fn node_id_to_point_id(node_id: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    node_id.hash(&mut hasher);
    hasher.finish()
}
