//! Auto-ingest: write grounded entities to Neo4j.
//!
//! This is the automated equivalent of `run_ingest()`, used by the process
//! endpoint. Instead of selecting items by `review_status = 'approved'`
//! (which requires manual human review), it selects items by
//! `grounding_status` — grounded items are written, ungrounded are flagged.
//!
//! ## Rust Learning: Reuse not duplication
//!
//! The Neo4j write logic (transaction, node creation, relationships, entity
//! resolution) is identical to `run_ingest`. We reuse ALL helper functions
//! from `ingest_helpers` and `ingest_resolver`. The only difference is the
//! data source query (grounding-based vs review-based).

use std::collections::HashMap;

use serde::Serialize;

use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository::{self, steps};
use crate::state::AppState;

use super::ingest_helpers::{
    create_contained_in_relationships, create_document_node, create_entity_node,
    create_ingest_relationship, create_party_nodes, create_provenance_relationships,
};
use super::ingest_resolver;

// ── Result DTO ─────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AutoIngestResult {
    pub entities_written: i32,
    pub entities_flagged: i32,
    pub relationships_written: i32,
    pub nodes_created: serde_json::Value,
    pub relationships_created: serde_json::Value,
    pub resolution_summary: serde_json::Value,
    pub duration_secs: f64,
}

// ── Core function ──────────────────────────────────────────────

/// Auto-ingest: write grounded entities to Neo4j.
///
/// Selects items by grounding_status instead of review_status. The Neo4j
/// transaction pattern (all-or-nothing) is identical to `run_ingest`.
/// After a successful write, marks items as 'written' or 'flagged' in
/// the `graph_status` column.
pub(crate) async fn run_auto_ingest(
    state: &AppState,
    doc_id: &str,
    username: &str,
) -> Result<AutoIngestResult, AppError> {
    let start = std::time::Instant::now();
    let step_id = steps::record_step_start(
        &state.pipeline_pool, doc_id, "ingest", username, &serde_json::json!({}),
    ).await.map_err(|e| AppError::Internal { message: format!("Step logging: {e}") })?;

    // 1. Fetch document
    let document = pipeline_repository::get_document(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    // 2. Find latest COMPLETED extraction run
    let run_id = pipeline_repository::get_latest_completed_run(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("No completed extraction run for document '{doc_id}'"),
        })?;

    // 3. Fetch GROUNDED items and their relationships (grounding-based selection)
    let items = pipeline_repository::get_grounded_items_for_document(
        &state.pipeline_pool, run_id,
    )
    .await
    .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;

    let relationships = pipeline_repository::get_grounded_relationships_for_document(
        &state.pipeline_pool, run_id,
    )
    .await
    .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;

    tracing::info!(
        doc_id = %doc_id, run_id, items = items.len(),
        rels = relationships.len(), "Fetched grounded extraction data"
    );

    // 4. Entity resolution — resolve Party items against existing Neo4j nodes
    let existing_parties = ingest_resolver::fetch_existing_parties(&state.graph).await?;
    let (resolution_map, resolution_summary) =
        ingest_resolver::resolve_parties(&items, &existing_parties).await?;

    tracing::info!(
        matched = resolution_summary.matched_existing,
        new = resolution_summary.created_new,
        "Entity resolution complete"
    );

    // 5. Open Neo4j transaction — all-or-nothing
    let mut txn = state.graph.start_txn().await.map_err(|e| AppError::Internal {
        message: format!("Failed to start Neo4j transaction: {e}"),
    })?;

    let mut pg_to_neo4j: HashMap<i32, String> = HashMap::new();
    let mut all_node_ids: Vec<String> = Vec::new();

    // 6. Create Document node
    let doc_type = document.document_type.clone();
    let doc_neo4j_id =
        create_document_node(&mut txn, doc_id, &document.title, &doc_type).await?;

    // 7. Create/merge Party nodes
    let mut pg_to_label: HashMap<i32, String> = HashMap::new();
    let (person_count, org_count) =
        create_party_nodes(&mut txn, &items, doc_id, &mut pg_to_neo4j, &mut pg_to_label, &resolution_map).await?;
    {
        let mut seen = std::collections::HashSet::new();
        for neo_id in pg_to_neo4j.values() {
            if seen.insert(neo_id.clone()) {
                all_node_ids.push(neo_id.clone());
            }
        }
    }

    // 8. Create all non-Party entity nodes
    let mut entity_type_counts: HashMap<String, usize> = HashMap::new();
    let mut entity_seq: HashMap<String, usize> = HashMap::new();

    for item in items.iter().filter(|i| i.entity_type != "Party") {
        let seq = entity_seq.entry(item.entity_type.clone()).or_insert(0);
        *seq += 1;

        let neo4j_id = create_entity_node(&mut txn, item, doc_id, *seq).await?;

        pg_to_neo4j.insert(item.id, neo4j_id.clone());
        all_node_ids.push(neo4j_id);

        *entity_type_counts.entry(item.entity_type.clone()).or_insert(0) += 1;
    }

    // 9. Create extraction relationships
    let mut rel_type_counts: HashMap<String, usize> = HashMap::new();

    for rel in &relationships {
        let from_neo = pg_to_neo4j.get(&rel.from_item_id).ok_or_else(|| {
            AppError::Internal {
                message: format!(
                    "No Neo4j ID for from_item_id {} (rel type {})",
                    rel.from_item_id, rel.relationship_type
                ),
            }
        })?;
        let to_neo = pg_to_neo4j.get(&rel.to_item_id).ok_or_else(|| {
            AppError::Internal {
                message: format!(
                    "No Neo4j ID for to_item_id {} (rel type {})",
                    rel.to_item_id, rel.relationship_type
                ),
            }
        })?;

        create_ingest_relationship(&mut txn, from_neo, to_neo, &rel.relationship_type).await?;

        *rel_type_counts.entry(rel.relationship_type.clone()).or_insert(0) += 1;
    }

    // 10. Create DERIVED_FROM provenance relationships
    let derived_from_count =
        create_provenance_relationships(&mut txn, &items, &pg_to_neo4j).await?;
    if derived_from_count > 0 {
        *rel_type_counts.entry("DERIVED_FROM".to_string()).or_insert(0) += derived_from_count;
    }

    // 11. Create CONTAINED_IN relationships
    let contained_in =
        create_contained_in_relationships(&mut txn, &all_node_ids, &doc_neo4j_id).await?;

    // 12. Commit transaction
    txn.commit().await.map_err(|e| AppError::Internal {
        message: format!("Neo4j transaction commit failed: {e}"),
    })?;

    // 13. Update pipeline document status → INGESTED
    //     (run_pipeline will then set COMPLETED after index + completeness)
    pipeline_repository::update_document_status(&state.pipeline_pool, doc_id, "INGESTED")
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update document status: {e}"),
        })?;

    // 14. Sync entity_type for Party → Person/Organization
    let mut entity_type_updates = 0usize;
    for item in &items {
        let actual_label = pg_to_label
            .get(&item.id)
            .map(|s| s.as_str())
            .unwrap_or(&item.entity_type);

        if actual_label != item.entity_type {
            pipeline_repository::update_item_entity_type(
                &state.pipeline_pool, item.id, actual_label,
            )
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to update entity_type for item {}: {e}", item.id),
            })?;
            entity_type_updates += 1;
        }
    }

    // 15. Mark items as written/flagged in graph_status column
    let (written, flagged) = pipeline_repository::update_graph_status_for_run(
        &state.pipeline_pool, run_id,
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to update graph_status: {e}"),
    })?;

    let duration = start.elapsed().as_secs_f64();
    let entity_node_total: usize = entity_type_counts.values().sum();
    let total_nodes = 1 + person_count + org_count + entity_node_total;
    let rel_total: usize = rel_type_counts.values().sum();
    let total_rels = rel_total + contained_in;
    let relationships_written = rel_total as i32;

    tracing::info!(
        doc_id = %doc_id, total_nodes, total_rels,
        written, flagged, entity_type_updates,
        duration_secs = format!("{duration:.2}"),
        "Auto-ingest complete"
    );

    log_admin_action(
        &state.audit_repo,
        username,
        "pipeline.document.auto_ingest",
        Some("document"),
        Some(doc_id),
        Some(serde_json::json!({
            "neo4j_document_id": doc_neo4j_id,
            "nodes": total_nodes,
            "relationships": total_rels,
            "entities_written": written,
            "entities_flagged": flagged,
        })),
    )
    .await;

    steps::record_step_complete(&state.pipeline_pool, step_id, duration, &serde_json::json!({
        "nodes_created": total_nodes, "relationships_created": total_rels,
        "entities_written": written, "entities_flagged": flagged,
        "derived_from": derived_from_count,
        "matched_existing": resolution_summary.matched_existing,
        "created_new": resolution_summary.created_new,
    })).await.ok();

    Ok(AutoIngestResult {
        entities_written: written,
        entities_flagged: flagged,
        relationships_written,
        nodes_created: serde_json::json!(entity_type_counts),
        relationships_created: serde_json::json!(rel_type_counts),
        resolution_summary: serde_json::to_value(&resolution_summary)
            .unwrap_or(serde_json::Value::Null),
        duration_secs: duration,
    })
}
