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

use crate::pipeline::constants::QDRANT_DOCUMENT_ID_FIELD;
use crate::repositories::embedding_repository::{self, EmbeddingRepoError};
use crate::services::embedding_service::{EmbeddingError, EmbeddingService};
use crate::services::embedding_text::build_embedding_text;
use crate::services::qdrant_payload;
use crate::services::qdrant_service::{self, QdrantError, QdrantPoint};

// STRUCTURAL: a memory bound, not a throughput setting.
//
// ## The 137 / 7.1G event, 2026-09-04
//
// A full admin reindex died on the DEV VM at 16:31 — `status=137/n/a`, which is
// SIGKILL, with a `7.1G memory peak` in the same journal line. It had deleted
// the Qdrant collection at 16:29, fetched 1,429 nodes, logged
// "Embedding 1429 texts..." and never logged again. The collection has been
// EMPTY since, so every gather has been blind.
//
// ## Why a batch of 50 became fatal when nothing about the batching changed
//
// Transformer attention is O(batch x length^2), and fastembed 4.9.1 pads every
// batch to its LONGEST member — `PaddingStrategy::BatchLongest` in
// `fastembed-4.9.1/src/common.rs:101`, not to the configured max. So the cost of
// a batch is set by its longest text, and 49 short texts do not make it cheaper.
//
// Before `.421`, `MAX_INPUT_TOKENS` was 512 and the tokenizer TRUNCATED there,
// so every batch was padded to at most 512 tokens no matter what it held —
// a ceiling nobody had to think about. `.421` raised it to 8192 (correct for
// L2a's queries), which removed the ceiling, and the padded length became
// whatever the corpus actually contains. Measured on DEV the same day: the
// longest built text is 2,345 characters, and 16 exceed 1,500.
//
// A batch of ONE cannot be padded to anything but its own length, so the peak
// becomes the cost of the single longest text rather than fifty times it. That
// is the whole of the fix: it removes the multiplier, and leaves the length
// alone. `MAX_INPUT_TOKENS` stays 8192 deliberately — truncating the corpus
// again to save memory would silently shorten what is searchable, which is the
// bug this project spent the day removing from the other half of retrieval.
//
// ## What 1 costs, stated honestly
//
// It is NOT free. ONNX Runtime parallelises within a batch, so a batch of 8
// really is faster than eight batches of 1 — what a larger batch multiplies is
// MEMORY, not time. 1 is the memory-safe FLOOR, not a universally optimal
// value: it is the only size that is safe without knowing both the host's spare
// RAM and the corpus's longest text, and this code knows neither.
//
// So this is a constant because the instruction that ordered the fix asked for
// one, and because an urgent fix to a step that currently cannot complete at
// all is the wrong moment to add an env var and the Ansible change it obliges.
// A deployment that knows its headroom and its text lengths could safely run 4
// or 8 and finish sooner. That is recorded as a ruling in
// CC_REPORT_EMBED_BATCH_OOM_v1, not decided here.
const EMBED_BATCH_SIZE: usize = 1;

// A liveness cadence for a human reading `journalctl`: 14 lines across the 1,429
// nodes on DEV — often enough to show the run is alive, rare enough not to bury
// the log.
//
// Deliberately NOT claimed as structural. A bigger corpus or a quieter journal
// could reasonably want a different number, so the honest position is that this
// is a config-shaped value carrying a compiled default, kept compiled because
// the instruction specified "every 100 texts" and this is an urgent fix. Raised
// as a ruling in CC_REPORT_EMBED_BATCH_OOM_v1 together with EMBED_BATCH_SIZE,
// since both are the same question.
const EMBED_PROGRESS_EVERY: usize = 100;

// ---------------------------------------------------------------------------
// Result and error types
// ---------------------------------------------------------------------------

/// Summary of a pipeline run, returned as the API response body.
#[derive(Debug)]
pub struct EmbeddingResult {
    pub total_nodes: usize,
    pub embedded_count: usize,
    /// Number of nodes skipped because they already exist in Qdrant.
    /// Always 0 in full (non-incremental) mode.
    pub skipped: usize,
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

/// Run the embedding pipeline:
/// 1. Ensure Qdrant collection exists
/// 2. Fetch all embeddable nodes from Neo4j
/// 3. (Incremental) Filter out nodes already in Qdrant
/// 4. (Dry-run) Report what would be embedded, then return
/// 5. Build embedding text for each node
/// 6. Generate embeddings via fastembed (in spawn_blocking)
/// 7. Upsert vectors + metadata to Qdrant
///
/// ## Parameters
/// - `incremental`: if true, only embed nodes whose `id` is not already
///   in Qdrant. When false (or after `--clean`), embeds everything.
/// - `dry_run`: if true, reports what would be embedded without actually
///   running fastembed or upserting to Qdrant.
pub async fn run_embedding_pipeline(
    graph: &Graph,
    http_client: &reqwest::Client,
    qdrant_url: &str,
    fastembed_cache_path: &str,
    incremental: bool,
    dry_run: bool,
    dimensions: u32,
) -> Result<EmbeddingResult, PipelineError> {
    let start = Instant::now();

    // Step 1: Ensure Qdrant collection
    qdrant_service::ensure_collection(http_client, qdrant_url, dimensions).await?;

    // Step 2: Fetch all nodes from Neo4j
    let nodes = embedding_repository::fetch_all_embeddable_nodes(graph).await?;
    let total_nodes = nodes.len();
    tracing::info!("Fetched {} nodes from Neo4j", total_nodes);

    if total_nodes == 0 {
        return Ok(EmbeddingResult {
            total_nodes: 0,
            embedded_count: 0,
            skipped: 0,
            nodes_by_type: HashMap::new(),
            duration_seconds: start.elapsed().as_secs_f64(),
            errors: vec![],
        });
    }

    // Step 3: Incremental filtering — skip nodes already in Qdrant.
    //
    // ## Rust Learning: Partition with retain vs. into_iter().filter()
    //
    // We use `retain()` on the Vec to filter in place. This avoids
    // allocating a second Vec. The `existing_ids` HashSet gives us
    // O(1) lookups, so the overall filter is O(n).
    let skipped;
    let mut nodes = nodes;

    if incremental {
        let existing_ids = qdrant_service::get_existing_point_ids(http_client, qdrant_url).await?;
        let before = nodes.len();
        nodes.retain(|n| !existing_ids.contains(&n.id));
        skipped = before - nodes.len();
        tracing::info!(
            "Incremental mode: {} new nodes to embed, {} already indexed, {} total in Neo4j",
            nodes.len(),
            skipped,
            total_nodes
        );
    } else {
        skipped = 0;
    }

    // Step 4: Dry-run — report what would be embedded, then exit early.
    if dry_run {
        tracing::info!("Dry-run mode: would embed {} nodes:", nodes.len());
        for node in &nodes {
            let title = node
                .properties
                .get("title")
                .or_else(|| node.properties.get("name"))
                .cloned()
                .unwrap_or_default();
            tracing::info!("  {} [{}] {}", node.id, node.node_type, title);
        }

        let mut nodes_by_type: HashMap<String, usize> = HashMap::new();
        for node in &nodes {
            *nodes_by_type.entry(node.node_type.clone()).or_insert(0) += 1;
        }

        return Ok(EmbeddingResult {
            total_nodes,
            embedded_count: 0,
            skipped,
            nodes_by_type,
            duration_seconds: start.elapsed().as_secs_f64(),
            errors: vec![],
        });
    }

    // Nothing new to embed — exit early without loading fastembed.
    if nodes.is_empty() {
        tracing::info!("No new nodes to embed — Qdrant is up to date");
        return Ok(EmbeddingResult {
            total_nodes,
            embedded_count: 0,
            skipped,
            nodes_by_type: HashMap::new(),
            duration_seconds: start.elapsed().as_secs_f64(),
            errors: vec![],
        });
    }

    // Step 5: Build embedding texts
    let texts: Vec<String> = nodes
        .iter()
        .map(|n| build_embedding_text(&n.node_type, &n.properties))
        .collect();

    // Count nodes by type (for the response)
    let mut nodes_by_type: HashMap<String, usize> = HashMap::new();
    for node in &nodes {
        *nodes_by_type.entry(node.node_type.clone()).or_insert(0) += 1;
    }

    // Step 6: Embed all texts via spawn_blocking
    // TextEmbedding is NOT Send, so we create it inside the blocking closure.
    tracing::info!("Embedding {} texts...", texts.len());
    let total_texts = texts.len();
    let cache_path = fastembed_cache_path.to_string();
    let vectors = tokio::task::spawn_blocking(move || {
        let mut service = EmbeddingService::new(&cache_path)?;
        let mut all_vectors = Vec::new();
        for (done, chunk) in texts.chunks(EMBED_BATCH_SIZE).enumerate() {
            let batch = chunk.to_vec();
            let embeddings = service.embed_batch(batch)?;
            all_vectors.extend(embeddings);

            // Liveness. A full corpus embed is minutes of silence otherwise, and
            // the last time it died the journal's final line was "Embedding 1429
            // texts..." followed by the kill — nothing to say how far it got.
            let embedded = (done + 1) * EMBED_BATCH_SIZE;
            if embedded.is_multiple_of(EMBED_PROGRESS_EVERY) {
                tracing::info!(
                    embedded = embedded.min(total_texts),
                    total = total_texts,
                    "embedding progress"
                );
            }
        }
        Ok::<Vec<Vec<f32>>, EmbeddingError>(all_vectors)
    })
    .await??;

    // Step 7: Build Qdrant points
    let mut points = Vec::new();
    let mut errors = Vec::new();

    for (i, node) in nodes.iter().enumerate() {
        let Some(vector) = vectors.get(i) else {
            errors.push(format!("Missing vector for node {}", node.id));
            continue;
        };

        // The required keys come from the SHARED builder — the same one the two
        // per-document index paths use — so a point written by this corpus
        // re-embed carries `document_id` and is deletable by exactly the filter
        // `qdrant_service::delete_points_by_filter` uses. Before this, it wrote
        // no `document_id` at all and one run would have left every point in the
        // collection undeletable by document.
        //
        // This path's own behaviour — copying the node's other properties in
        // wholesale — LAYERS ON TOP. It is what makes the re-embed's payload a
        // superset of the per-document one, and it stays.
        let document_id = node.properties.get(QDRANT_DOCUMENT_ID_FIELD);
        if document_id.is_none() {
            // Never an empty string: a point stored with `document_id: ""` would
            // match a filter for the empty string and be undeletable by its real
            // document for ever. Omitted and named instead.
            tracing::warn!(
                node_id = %node.id,
                node_type = %node.node_type,
                "embedding: node has no document linkage — its point will carry no \
                 document_id and will not be removed when its document is deleted"
            );
        }
        let mut payload =
            qdrant_payload::build_point_payload(node, document_id.map(String::as_str));

        if let Some(obj) = payload.as_object_mut() {
            for (key, value) in &node.properties {
                // `title`/`name` feed the builder's title; `document_id` is a key
                // the builder owns. Re-inserting them here would let this loop
                // silently overrule the shared builder, which is the divergence
                // this whole change exists to end.
                if key == "title" || key == "name" || key == QDRANT_DOCUMENT_ID_FIELD {
                    continue;
                }
                obj.insert(key.clone(), serde_json::Value::String(value.clone()));
            }
        }

        points.push(QdrantPoint {
            id: node_id_to_point_id(&node.id),
            vector: vector.clone(),
            payload,
        });
    }

    let embedded_count = points.len();

    // Step 8: Upsert to Qdrant
    tracing::info!("Upserting {} points to Qdrant...", embedded_count);
    qdrant_service::upsert_points(http_client, qdrant_url, points).await?;

    let duration = start.elapsed().as_secs_f64();
    tracing::info!("Pipeline complete in {:.1}s", duration);

    Ok(EmbeddingResult {
        total_nodes,
        embedded_count,
        skipped,
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
