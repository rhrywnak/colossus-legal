//! Chunk extraction orchestration — splits text, runs sequential per-chunk
//! extraction with inter-chunk delay, merges results, persists per-chunk
//! observability rows, and stores merged entities/relationships.
//!
//! ## Rate Limiting
//!
//! Anthropic's API has an 8,000 output tokens/minute rate limit. Concurrent
//! extraction (even with CONCURRENCY_LIMIT=2) can exceed this in seconds.
//! We process chunks sequentially with a 15-second delay between each to
//! stay well under the limit.

use std::time::Instant;

use colossus_extract::{
    ChunkExtractionResult, ExtractedNode, ExtractedRel, FixedSizeSplitter,
    TextChunk, TextSplitter,
};
use sqlx::PgPool;

use crate::error::AppError;
use crate::repositories::pipeline_repository::documents;

/// Delay between sequential chunk extractions to avoid rate limits.
const INTER_CHUNK_DELAY_SECS: u64 = 15;


/// Summary returned from the chunk extraction run.
pub(super) struct ChunkExtractionSummary {
    /// Merged entities + relationships in the legacy JSON shape consumed by
    /// `store_entities_and_relationships` and `validate_completeness`.
    pub legacy_json: serde_json::Value,
    pub chunk_count: usize,
    pub chunks_succeeded: usize,
    pub chunks_failed: usize,
}

/// Split `full_text` into chunks and extract each sequentially with a delay,
/// writing a row into `extraction_chunks` per chunk (success or failure).
///
/// Updates document progress after each chunk completes so the frontend
/// can display per-chunk progress during PROCESSING status.
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_chunk_extraction(
    pool: &PgPool,
    run_id: i32,
    doc_id: &str,
    full_text: &str,
    schema_json: &serde_json::Value,
    prompt_template: &str,
    // TODO(Phase4): replaced by context.llm_provider.invoke() in LlmExtract step
) -> Result<ChunkExtractionSummary, AppError> {
    let chunks: Vec<TextChunk> = FixedSizeSplitter::new().split(full_text);
    let chunk_count = chunks.len();
    tracing::info!(run_id, chunk_count, "Split document into chunks");

    let mut merged_nodes: Vec<ExtractedNode> = Vec::new();
    let mut merged_rels: Vec<ExtractedRel> = Vec::new();
    let mut chunks_succeeded = 0usize;
    let mut chunks_failed = 0usize;

    // Sequential extraction with inter-chunk delay to stay under rate limits.
    // Concurrent extraction disabled due to Anthropic rate limits (8k output tokens/min).
    // Re-enable when rate limit is increased or when using a self-hosted model.
    for (index, chunk) in chunks.iter().enumerate() {
        // Update progress before starting this chunk
        let entities_so_far = merged_nodes.len();
        let pct = 10 + (50 * (index) / chunk_count.max(1));
        documents::update_processing_progress(
            pool, doc_id, "extract",
            &format!("Analyzing content... chunk {} of {}", index + 1, chunk_count),
            chunk_count as i32, index as i32,
            entities_so_far as i32, pct as i32,
        ).await.ok();

        // Extract this chunk with rate limit retry loop.
        //
        // ## Why the retry loop lives here, not in the extractor
        //
        // The extractor's job is to make one HTTP call and return a typed result.
        // The orchestrator's job is to decide what to do with that result — including
        // how many times to retry and what to tell the user during waits.
        //
        // The orchestrator has two things the extractor doesn't:
        // 1. Access to the database pool (needed to update the progress label)
        // 2. Context about which chunk this is (chunk index, total count)
        //
        // These two things together let us show the user exactly what is happening:
        // "Rate limited — resuming in 45s (chunk 4 of 8)"
        // instead of a frozen progress bar with no explanation.
        let start = Instant::now();

        // TODO(Phase4): replaced by context.llm_provider.invoke() in LlmExtract step
        let result: Result<ChunkExtractionResult, colossus_extract::PipelineError> = Err(
            colossus_extract::PipelineError::Extraction(
                "chunk extractor removed — pending Phase 4 LlmExtract step".into(),
            ),
        );
        let duration_ms = start.elapsed().as_millis() as i64;

        match result {
            Ok(mut chunk_result) => {
                prefix_chunk_ids(&mut chunk_result, chunk.index);
                let node_count = chunk_result.nodes.len();
                let rel_count = chunk_result.relationships.len();
                merged_nodes.extend(chunk_result.nodes);
                merged_rels.extend(chunk_result.relationships);
                chunks_succeeded += 1;
                insert_chunk_row(
                    pool, run_id, chunk.index as i32, &chunk.text,
                    "completed", node_count as i32, rel_count as i32,
                    None, duration_ms,
                ).await;
                tracing::info!(
                    run_id, chunk_index = chunk.index, node_count, rel_count,
                    duration_ms, "Chunk extraction succeeded"
                );
            }
            Err(err) => {
                chunks_failed += 1;
                let error_message = format!("{err:?}");
                tracing::error!(
                    run_id, chunk_index = chunk.index,
                    error = %error_message, "Chunk extraction failed"
                );
                insert_chunk_row(
                    pool, run_id, chunk.index as i32, &chunk.text,
                    "failed", 0, 0, Some(&error_message), duration_ms,
                ).await;
            }
        }

        // Delay between chunks (skip delay after last chunk)
        if index < chunks.len() - 1 {
            tracing::info!(
                run_id, delay_secs = INTER_CHUNK_DELAY_SECS,
                "Waiting between chunks to respect rate limits"
            );
            tokio::time::sleep(tokio::time::Duration::from_secs(INTER_CHUNK_DELAY_SECS)).await;
        }
    }

    // Final progress update
    let final_pct = 55;
    documents::update_processing_progress(
        pool, doc_id, "extract",
        "Content analyzed",
        chunk_count as i32, chunk_count as i32,
        merged_nodes.len() as i32, final_pct,
    ).await.ok();

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
pub(crate) fn chunk_results_to_legacy_json(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_json_promotes_verbatim_quote() {
        use colossus_extract::ExtractedNode;
        use std::collections::HashMap;

        // Build a node with verbatim_quote in properties (as LLM returns it)
        let mut properties = HashMap::new();
        properties.insert(
            "verbatim_quote".to_string(),
            serde_json::Value::String("Plaintiff was wrongfully terminated.".to_string()),
        );
        properties.insert(
            "paragraph_number".to_string(),
            serde_json::Value::String("42".to_string()),
        );

        let node = ExtractedNode {
            id: "chunk_0:1".to_string(),
            label: "ComplaintAllegation".to_string(),
            properties,
        };

        let legacy = chunk_results_to_legacy_json(&[node], &[]);

        let entities = legacy["entities"].as_array().unwrap();
        assert_eq!(entities.len(), 1);

        let entity = &entities[0];

        // verbatim_quote must be promoted to the top level
        assert_eq!(
            entity["verbatim_quote"].as_str(),
            Some("Plaintiff was wrongfully terminated."),
            "verbatim_quote must be promoted from properties to top-level entity"
        );
    }

    #[test]
    fn test_legacy_json_node_without_verbatim_quote() {
        use colossus_extract::ExtractedNode;
        use std::collections::HashMap;

        // Party node — no verbatim_quote
        let mut properties = HashMap::new();
        properties.insert(
            "full_name".to_string(),
            serde_json::Value::String("Marie Awad".to_string()),
        );

        let node = ExtractedNode {
            id: "chunk_0:0".to_string(),
            label: "Party".to_string(),
            properties,
        };

        let legacy = chunk_results_to_legacy_json(&[node], &[]);
        let entity = &legacy["entities"][0];

        // verbatim_quote should not appear at top level when not in properties
        assert!(
            entity["verbatim_quote"].is_null(),
            "Entity without verbatim_quote should not have null promoted to top level"
        );
    }

    #[test]
    fn test_legacy_json_preserves_entity_type_and_id() {
        use colossus_extract::ExtractedNode;
        use std::collections::HashMap;

        let node = ExtractedNode {
            id: "chunk_2:5".to_string(),
            label: "LegalCount".to_string(),
            properties: HashMap::new(),
        };

        let legacy = chunk_results_to_legacy_json(&[node], &[]);
        let entity = &legacy["entities"][0];

        assert_eq!(entity["entity_type"].as_str(), Some("LegalCount"));
        assert_eq!(entity["id"].as_str(), Some("chunk_2:5"));
    }

}
