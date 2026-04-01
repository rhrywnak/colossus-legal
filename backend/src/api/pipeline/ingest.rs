//! POST /api/admin/pipeline/documents/:id/ingest — Graph Writer.
//!
//! Reads verified extraction items from pipeline DB and writes them as
//! nodes and relationships into Neo4j. Uses entity resolution for parties.
//!
//! ## Rust Learning: HashMap<i32, String> maps PG item IDs → Neo4j string IDs.
//! Built during node creation; used for relationships. All nodes must be
//! created before relationships so the map is fully populated.

use std::collections::HashMap;

use axum::{extract::Path, extract::State, Json};
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository::{self, steps};
use crate::state::AppState;

use super::ingest_helpers::{
    create_allegation_nodes, create_contained_in_relationships, create_count_nodes,
    create_document_node, create_harm_nodes, create_ingest_relationship, create_party_nodes,
};
use super::ingest_resolver::{self, ResolutionSummary};

// ── Response DTOs ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub document_id: String,
    pub status: String,
    pub neo4j_document_id: String,
    pub nodes_created: NodeCounts,
    pub relationships_created: RelCounts,
    pub resolution_summary: ResolutionSummary,
    pub duration_secs: f64,
}

#[derive(Debug, Serialize)]
pub struct NodeCounts {
    pub document: usize,
    pub person: usize,
    pub organization: usize,
    pub complaint_allegation: usize,
    pub legal_count: usize,
    pub harm: usize,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct RelCounts {
    pub stated_by: usize,
    pub about: usize,
    pub supports: usize,
    pub contained_in: usize,
    pub total: usize,
}

// ── Handler ─────────────────────────────────────────────────────

/// POST /api/admin/pipeline/documents/:id/ingest
///
/// Writes verified extraction data from pipeline DB into Neo4j as a
/// knowledge graph. All Neo4j writes happen in a single transaction —
/// if anything fails, the entire import rolls back.
pub async fn ingest_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<IngestResponse>, AppError> {
    require_admin(&user)?;
    let start = std::time::Instant::now();
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST ingest");
    let step_id = steps::record_step_start(
        &state.pipeline_pool, &doc_id, "ingest", &user.username, &serde_json::json!({}),
    ).await.map_err(|e| AppError::Internal { message: format!("Step logging: {e}") })?;

    // 1. Fetch document — must exist
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    // 2. Verify status = VERIFIED
    if document.status != "VERIFIED" {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot ingest: status is '{}', expected 'VERIFIED'",
                document.status
            ),
            details: serde_json::json!({ "status": document.status }),
        });
    }

    // 3. Find latest COMPLETED extraction run
    let run_id = pipeline_repository::get_latest_completed_run(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("No completed extraction run for document '{doc_id}'"),
        })?;

    // 4. Fetch items and relationships for that run
    let items = pipeline_repository::get_items_for_run(&state.pipeline_pool, run_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;

    let relationships = pipeline_repository::get_relationships_for_run(&state.pipeline_pool, run_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;

    tracing::info!(
        doc_id = %doc_id, run_id, items = items.len(),
        rels = relationships.len(), "Fetched extraction data"
    );

    // 5. Entity resolution — resolve Party items against existing Neo4j nodes
    let existing_parties = ingest_resolver::fetch_existing_parties(&state.graph).await?;
    tracing::info!(existing = existing_parties.len(), "Fetched existing parties for resolution");

    let (resolution_map, resolution_summary) =
        ingest_resolver::resolve_parties(&items, &existing_parties).await?;

    tracing::info!(
        matched = resolution_summary.matched_existing,
        new = resolution_summary.created_new,
        "Entity resolution complete"
    );

    // 7. Open Neo4j transaction — all-or-nothing
    let mut txn = state.graph.start_txn().await.map_err(|e| AppError::Internal {
        message: format!("Failed to start Neo4j transaction: {e}"),
    })?;

    // PG item ID → Neo4j node ID mapping (populated during node creation)
    let mut pg_to_neo4j: HashMap<i32, String> = HashMap::new();
    // Collect all non-Document node IDs for CONTAINED_IN relationships
    let mut all_node_ids: Vec<String> = Vec::new();

    // 8. Create Document node
    let doc_type = pipeline_repository::get_pipeline_config(&state.pipeline_pool, &doc_id)
        .await
        .ok()
        .flatten()
        .map(|c| c.schema_file)
        .unwrap_or_else(|| document.document_type.clone());

    let doc_neo4j_id =
        create_document_node(&mut txn, &doc_id, &document.title, &doc_type).await?;

    // 9. Create/merge Party nodes (Person + Organization) using resolution map
    let (person_count, org_count) =
        create_party_nodes(&mut txn, &items, &doc_id, &mut pg_to_neo4j, &resolution_map).await?;
    // Collect unique party node IDs for CONTAINED_IN
    {
        let mut seen = std::collections::HashSet::new();
        for neo_id in pg_to_neo4j.values() {
            if seen.insert(neo_id.clone()) {
                all_node_ids.push(neo_id.clone());
            }
        }
    }

    // 10. Create ComplaintAllegation nodes
    let allegation_count =
        create_allegation_nodes(&mut txn, &items, &doc_id, &mut pg_to_neo4j).await?;
    // Collect allegation node IDs
    for item in items.iter().filter(|i| i.entity_type == "FactualAllegation") {
        if let Some(neo_id) = pg_to_neo4j.get(&item.id) {
            all_node_ids.push(neo_id.clone());
        }
    }

    // 11. Create LegalCount nodes
    let count_count =
        create_count_nodes(&mut txn, &items, &doc_id, &mut pg_to_neo4j).await?;
    for item in items.iter().filter(|i| i.entity_type == "LegalCount") {
        if let Some(neo_id) = pg_to_neo4j.get(&item.id) {
            all_node_ids.push(neo_id.clone());
        }
    }

    // 12. Create Harm nodes
    let harm_count =
        create_harm_nodes(&mut txn, &items, &doc_id, &mut pg_to_neo4j).await?;
    for item in items.iter().filter(|i| i.entity_type == "DamagesClaim") {
        if let Some(neo_id) = pg_to_neo4j.get(&item.id) {
            all_node_ids.push(neo_id.clone());
        }
    }

    // 13. Create extraction relationships (STATED_BY, ABOUT, SUPPORTS)
    let (mut stated_by, mut about, mut supports) = (0usize, 0usize, 0usize);

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

        match rel.relationship_type.as_str() {
            "STATED_BY" => stated_by += 1,
            "ABOUT" => about += 1,
            "SUPPORTS" => supports += 1,
            other => {
                tracing::warn!(rel_type = other, "Unknown relationship type — created anyway");
                // Count it as the closest match; won't break anything
            }
        }
    }

    // 14. Create CONTAINED_IN relationships (all nodes → Document)
    let contained_in =
        create_contained_in_relationships(&mut txn, &all_node_ids, &doc_neo4j_id).await?;

    // 15. Commit transaction
    txn.commit().await.map_err(|e| AppError::Internal {
        message: format!("Neo4j transaction commit failed: {e}"),
    })?;

    // 16. Update pipeline document status → INGESTED
    pipeline_repository::update_document_status(&state.pipeline_pool, &doc_id, "INGESTED")
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update document status: {e}"),
        })?;

    let duration = start.elapsed().as_secs_f64();
    let total_nodes = 1 + person_count + org_count + allegation_count + count_count + harm_count;
    let total_rels = stated_by + about + supports + contained_in;

    tracing::info!(
        doc_id = %doc_id, total_nodes, total_rels,
        duration_secs = format!("{duration:.2}"),
        "Ingest complete"
    );

    log_admin_action(
        &state.audit_repo,
        &user.username,
        "pipeline.document.ingest",
        Some("document"),
        Some(&doc_id),
        Some(serde_json::json!({
            "neo4j_document_id": doc_neo4j_id,
            "nodes": total_nodes,
            "relationships": total_rels,
        })),
    )
    .await;

    steps::record_step_complete(&state.pipeline_pool, step_id, duration, &serde_json::json!({
        "nodes_created": total_nodes, "relationships_created": total_rels,
        "matched_existing": resolution_summary.matched_existing, "created_new": resolution_summary.created_new,
    })).await.ok();
    Ok(Json(IngestResponse {
        document_id: doc_id,
        status: "INGESTED".to_string(),
        neo4j_document_id: doc_neo4j_id,
        nodes_created: NodeCounts {
            document: 1,
            person: person_count,
            organization: org_count,
            complaint_allegation: allegation_count,
            legal_count: count_count,
            harm: harm_count,
            total: total_nodes,
        },
        relationships_created: RelCounts {
            stated_by,
            about,
            supports,
            contained_in,
            total: total_rels,
        },
        resolution_summary,
        duration_secs: duration,
    }))
}
