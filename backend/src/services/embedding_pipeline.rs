//! Embedding pipeline orchestrator.
//!
//! Ties together: Neo4j fetch → text building → fastembed → Qdrant upsert.
//!
//! ## Pattern: Graceful error accumulation
//! Instead of failing the entire pipeline when one node has a problem,
//! we collect errors into a `Vec<String>` and continue processing.
//! The final result includes both the success count and the error list,
//! so the caller can see what worked and what didn't.
//!
//! ## Pattern: Instant::now() + elapsed()
//! `std::time::Instant` is a monotonic clock — it only goes forward and
//! isn't affected by system clock adjustments. `instant.elapsed()` returns
//! a `Duration` which we convert to seconds for the response.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::Instant;

use neo4rs::Graph;

use crate::repositories::embedding_repository::{self, EmbeddingRepoError};
use crate::services::embedding_service::{EmbeddingError, EmbeddingService};
use crate::services::embedding_text::build_embedding_text;
use crate::services::qdrant_service::{self, QdrantError, QdrantPoint};

// ---------------------------------------------------------------------------
// Result and error types
// ---------------------------------------------------------------------------

/// Summary of a pipeline run, returned as the API response body.
#[derive(Debug)]
pub struct EmbeddingResult {
    pub total_nodes: usize,
    pub embedded_count: usize,
    pub nodes_by_type: HashMap<String, usize>,
    pub duration_seconds: f64,
    pub errors: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Qdrant error: {0}")]
    Qdrant(#[from] QdrantError),

    #[error("Neo4j repository error: {0}")]
    Repository(#[from] EmbeddingRepoError),

    #[error("Embedding error: {0}")]
    Embedding(#[from] EmbeddingError),

    #[error("Blocking task panicked")]
    JoinError(#[from] tokio::task::JoinError),
}

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

/// Run the full embedding pipeline:
/// 1. Ensure Qdrant collection exists
/// 2. Fetch all embeddable nodes from Neo4j
/// 3. Build embedding text for each node
/// 4. Generate embeddings via fastembed (in spawn_blocking)
/// 5. Upsert vectors + metadata to Qdrant
pub async fn run_embedding_pipeline(
    graph: &Graph,
    http_client: &reqwest::Client,
    qdrant_url: &str,
    fastembed_cache_path: &str,
) -> Result<EmbeddingResult, PipelineError> {
    let start = Instant::now();

    // Step 1: Ensure Qdrant collection
    qdrant_service::ensure_collection(http_client, qdrant_url).await?;

    // Step 2: Fetch all nodes from Neo4j
    let nodes = embedding_repository::fetch_all_embeddable_nodes(graph).await?;
    let total_nodes = nodes.len();
    tracing::info!("Fetched {} nodes from Neo4j", total_nodes);

    if total_nodes == 0 {
        return Ok(EmbeddingResult {
            total_nodes: 0,
            embedded_count: 0,
            nodes_by_type: HashMap::new(),
            duration_seconds: start.elapsed().as_secs_f64(),
            errors: vec![],
        });
    }

    // Step 3: Build embedding texts
    let texts: Vec<String> = nodes
        .iter()
        .map(|n| build_embedding_text(&n.node_type, &n.properties))
        .collect();

    // Count nodes by type (for the response)
    let mut nodes_by_type: HashMap<String, usize> = HashMap::new();
    for node in &nodes {
        *nodes_by_type.entry(node.node_type.clone()).or_insert(0) += 1;
    }

    // Step 4: Embed all texts via spawn_blocking
    // TextEmbedding is NOT Send, so we create it inside the blocking closure.
    tracing::info!("Embedding {} texts...", texts.len());
    let cache_path = fastembed_cache_path.to_string();
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
    .await??;

    // Step 5: Build Qdrant points
    let mut points = Vec::new();
    let mut errors = Vec::new();

    for (i, node) in nodes.iter().enumerate() {
        // Safety: vectors.len() should equal nodes.len()
        let Some(vector) = vectors.get(i) else {
            errors.push(format!("Missing vector for node {}", node.id));
            continue;
        };

        let mut payload = serde_json::json!({
            "node_id": node.id,
            "node_type": node.node_type,
            "title": node.properties.get("title")
                .or_else(|| node.properties.get("name"))
                .unwrap_or(&String::new()),
        });

        // Include document_id and page_number if present (for Evidence nodes)
        if let Some(doc_id) = node.properties.get("document_id") {
            payload["document_id"] = serde_json::Value::String(doc_id.clone());
        }
        if let Some(page) = node.properties.get("page_number") {
            payload["page_number"] = serde_json::Value::String(page.clone());
        }

        points.push(QdrantPoint {
            id: node_id_to_point_id(&node.id),
            vector: vector.clone(),
            payload,
        });
    }

    let embedded_count = points.len();

    // Step 6: Upsert to Qdrant
    tracing::info!("Upserting {} points to Qdrant...", embedded_count);
    qdrant_service::upsert_points(http_client, qdrant_url, points).await?;

    let duration = start.elapsed().as_secs_f64();
    tracing::info!("Pipeline complete in {:.1}s", duration);

    Ok(EmbeddingResult {
        total_nodes,
        embedded_count,
        nodes_by_type,
        duration_seconds: duration,
        errors,
    })
}

/// Convert a node ID string to a deterministic u64 for Qdrant point IDs.
///
/// ## Pattern: DefaultHasher for deterministic hashing
/// `DefaultHasher` produces a consistent u64 hash within a single Rust
/// version. We don't need cross-version stability — if the hash changes
/// after a Rust update, we just re-run the pipeline and it overwrites
/// the old points. This is safe because the pipeline always does a full
/// re-embed of all nodes.
fn node_id_to_point_id(node_id: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    node_id.hash(&mut hasher);
    hasher.finish()
}
