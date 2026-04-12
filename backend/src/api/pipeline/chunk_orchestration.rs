//! Chunk extraction orchestration — splits text, runs concurrent per-chunk
//! extraction, merges results, persists per-chunk observability rows, and
//! stores merged entities/relationships via the existing repo helpers.
//!
//! This module exists because `extract.rs` would otherwise exceed the
//! 300-line module limit (CLAUDE.md golden rule). The handler remains
//! the orchestrator of overall flow (validation, run recording, status
//! transitions); this module owns the chunk-level concurrency, merging,
//! and persistence glue.

use std::sync::Arc;
use std::time::Instant;

use colossus_extract::{
    ChunkExtractionResult, ChunkExtractor, ExtractedNode, ExtractedRel, FixedSizeSplitter,
    TextChunk, TextSplitter,
};
use futures::future::join_all;
use sqlx::PgPool;
use tokio::sync::Semaphore;

use super::chunk_extractor::AnthropicChunkExtractor;
use crate::error::AppError;

/// Maximum chunks in flight against the LLM API at once.
const CONCURRENCY_LIMIT: usize = 2;

/// Summary returned from the chunk extraction run.
pub(super) struct ChunkExtractionSummary {
    /// Merged entities + relationships in the legacy JSON shape consumed by
    /// `store_entities_and_relationships` and `validate_completeness`.
    pub legacy_json: serde_json::Value,
    pub chunk_count: usize,
    pub chunks_succeeded: usize,
    pub chunks_failed: usize,
}

/// Split `full_text` into chunks and extract each concurrently, writing a
/// row into `extraction_chunks` per chunk (success or failure).
///
/// The prompt template and schema JSON are passed through verbatim to the
/// chunk extractor; FP-6 will update the template files to include the
/// `{{chunk_text}}` / `{{schema_json}}` placeholders the extractor substitutes.
pub(super) async fn run_chunk_extraction(
    pool: &PgPool,
    run_id: i32,
    full_text: &str,
    schema_json: &serde_json::Value,
    prompt_template: &str,
    extractor: Arc<AnthropicChunkExtractor>,
) -> Result<ChunkExtractionSummary, AppError> {
    let chunks: Vec<TextChunk> = FixedSizeSplitter::new().split(full_text);
    let chunk_count = chunks.len();
    tracing::info!(run_id, chunk_count, "Split document into chunks");

    let sem = Arc::new(Semaphore::new(CONCURRENCY_LIMIT));
    let schema_json = Arc::new(schema_json.clone());
    let template = Arc::new(prompt_template.to_string());

    let mut handles = Vec::with_capacity(chunk_count);
    for chunk in chunks {
        let sem = Arc::clone(&sem);
        let extractor = Arc::clone(&extractor);
        let schema_json = Arc::clone(&schema_json);
        let template = Arc::clone(&template);

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire_owned().await.expect("semaphore closed");
            let start = Instant::now();
            let result = extractor
                .extract_chunk(&chunk.text, &schema_json, &template, "")
                .await;
            let duration_ms = start.elapsed().as_millis() as i64;
            (chunk.index, chunk.text, result, duration_ms)
        }));
    }

    let joined = join_all(handles).await;

    let mut merged_nodes: Vec<ExtractedNode> = Vec::new();
    let mut merged_rels: Vec<ExtractedRel> = Vec::new();
    let mut chunks_succeeded = 0usize;
    let mut chunks_failed = 0usize;

    for join_result in joined {
        match join_result {
            Ok((chunk_index, chunk_text, Ok(mut result), duration_ms)) => {
                prefix_chunk_ids(&mut result, chunk_index);
                let node_count = result.nodes.len();
                let rel_count = result.relationships.len();
                merged_nodes.extend(result.nodes);
                merged_rels.extend(result.relationships);
                chunks_succeeded += 1;
                insert_chunk_row(
                    pool,
                    run_id,
                    chunk_index as i32,
                    &chunk_text,
                    "completed",
                    node_count as i32,
                    rel_count as i32,
                    None,
                    duration_ms,
                )
                .await;
            }
            Ok((chunk_index, chunk_text, Err(err), duration_ms)) => {
                chunks_failed += 1;
                let error_message = format!("{err:?}");
                tracing::error!(
                    run_id,
                    chunk_index,
                    error = %error_message,
                    "Chunk extraction failed"
                );
                insert_chunk_row(
                    pool,
                    run_id,
                    chunk_index as i32,
                    &chunk_text,
                    "failed",
                    0,
                    0,
                    Some(&error_message),
                    duration_ms,
                )
                .await;
            }
            Err(join_err) => {
                // Task panicked — no chunk_index / chunk_text available.
                chunks_failed += 1;
                tracing::error!(run_id, error = %join_err, "Chunk extraction task panicked");
            }
        }
    }

    let legacy_json = chunk_results_to_legacy_json(&merged_nodes, &merged_rels);

    Ok(ChunkExtractionSummary {
        legacy_json,
        chunk_count,
        chunks_succeeded,
        chunks_failed,
    })
}

/// Prefix every node/relationship id with `chunk_{index}:` so merged IDs
/// are globally unique across chunks.
fn prefix_chunk_ids(result: &mut ChunkExtractionResult, chunk_index: usize) {
    let prefix = format!("chunk_{chunk_index}:");
    for node in &mut result.nodes {
        node.id = format!("{prefix}{}", node.id);
    }
    for rel in &mut result.relationships {
        rel.start_node_id = format!("{prefix}{}", rel.start_node_id);
        rel.end_node_id = format!("{prefix}{}", rel.end_node_id);
    }
}

/// Convert merged chunk results into the JSON shape expected by the
/// existing `store_entities_and_relationships` / `validate_completeness`
/// code paths — `{ entities: [...], relationships: [...] }`.
fn chunk_results_to_legacy_json(
    merged_nodes: &[ExtractedNode],
    merged_rels: &[ExtractedRel],
) -> serde_json::Value {
    let entities: Vec<serde_json::Value> = merged_nodes
        .iter()
        .map(|node| {
            let props_value = serde_json::to_value(&node.properties)
                .unwrap_or(serde_json::Value::Null);
            let mut entity = serde_json::json!({
                "entity_type": node.label,
                "id": node.id,
                "properties": props_value,
            });
            if let Some(quote) = node.properties.get("verbatim_quote") {
                entity["verbatim_quote"] = quote.clone();
            }
            entity
        })
        .collect();

    let relationships: Vec<serde_json::Value> = merged_rels
        .iter()
        .map(|rel| {
            let props_value = serde_json::to_value(&rel.properties)
                .unwrap_or(serde_json::Value::Null);
            serde_json::json!({
                "relationship_type": rel.rel_type,
                "from_entity": rel.start_node_id,
                "to_entity": rel.end_node_id,
                "properties": props_value,
            })
        })
        .collect();

    serde_json::json!({
        "entities": entities,
        "relationships": relationships,
    })
}

/// Insert a row into `extraction_chunks`. Best-effort — logs on failure
/// but does not abort the extraction.
#[allow(clippy::too_many_arguments)]
async fn insert_chunk_row(
    pool: &PgPool,
    run_id: i32,
    chunk_index: i32,
    chunk_text: &str,
    status: &str,
    node_count: i32,
    relationship_count: i32,
    error_message: Option<&str>,
    duration_ms: i64,
) {
    let res = sqlx::query(
        "INSERT INTO extraction_chunks \
         (extraction_run_id, chunk_index, chunk_text, status, \
          node_count, relationship_count, error_message, duration_ms) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(run_id)
    .bind(chunk_index)
    .bind(chunk_text)
    .bind(status)
    .bind(node_count)
    .bind(relationship_count)
    .bind(error_message)
    .bind(duration_ms as i32)
    .execute(pool)
    .await;

    if let Err(e) = res {
        tracing::error!(
            run_id,
            chunk_index,
            error = %e,
            "Failed to insert extraction_chunks row"
        );
    }
}

/// Update the per-run chunk statistics columns. Best-effort.
pub(super) async fn update_run_chunk_stats(
    pool: &PgPool,
    run_id: i32,
    chunk_count: usize,
    chunks_succeeded: usize,
    chunks_failed: usize,
) {
    let res = sqlx::query(
        "UPDATE extraction_runs \
         SET chunk_count = $1, chunks_succeeded = $2, chunks_failed = $3 \
         WHERE id = $4",
    )
    .bind(chunk_count as i32)
    .bind(chunks_succeeded as i32)
    .bind(chunks_failed as i32)
    .bind(run_id)
    .execute(pool)
    .await;

    if let Err(e) = res {
        tracing::error!(run_id, error = %e, "Failed to update extraction_runs chunk stats");
    }
}

