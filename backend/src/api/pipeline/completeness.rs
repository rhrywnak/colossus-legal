//! GET /api/admin/pipeline/documents/:id/completeness — Completeness Check.
//!
//! Cross-references extraction data in the pipeline PostgreSQL database
//! against what was written to Neo4j and indexed in Qdrant. Reports any
//! discrepancies. Updates document status to PUBLISHED if everything matches.

use std::collections::HashMap;

use axum::{extract::Path, extract::State, Json};
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository;
use crate::services::qdrant_service;
use crate::state::AppState;

use super::completeness_helpers::{
    check_entity_count, count_neo4j_nodes, count_neo4j_relationships, find_orphaned_nodes,
};

// ── Response DTOs ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CompletenessResponse {
    pub document_id: String,
    pub status: String,
    pub pipeline_db: PipelineCounts,
    pub neo4j: Neo4jCounts,
    pub qdrant: QdrantCounts,
    pub checks: Vec<CompletenessCheck>,
    pub published: bool,
}

#[derive(Debug, Serialize)]
pub struct PipelineCounts {
    pub extraction_items: HashMap<String, usize>,
    pub extraction_relationships: HashMap<String, usize>,
    pub total_items: usize,
    pub total_relationships: usize,
}

#[derive(Debug, Serialize)]
pub struct Neo4jCounts {
    pub nodes_by_label: HashMap<String, usize>,
    pub relationships_by_type: HashMap<String, usize>,
    pub total_nodes: usize,
    pub total_relationships: usize,
    pub orphaned_nodes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct QdrantCounts {
    pub total_points: usize,
    pub collection: String,
}

#[derive(Debug, Serialize)]
pub struct CompletenessCheck {
    pub name: String,
    pub status: String,
    pub expected: usize,
    pub actual: usize,
    pub message: String,
}

// ── Handler ─────────────────────────────────────────────────────

/// GET /api/admin/pipeline/documents/:id/completeness
pub async fn completeness_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<CompletenessResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "GET completeness");

    // 1. Fetch document — must exist
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    // 2. Verify status = INDEXED (or already PUBLISHED for re-check)
    if document.status != "INDEXED" && document.status != "PUBLISHED" {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot check completeness: status is '{}', expected 'INDEXED'",
                document.status
            ),
            details: serde_json::json!({ "status": document.status }),
        });
    }

    // 3. Get latest completed extraction run
    let run_id = pipeline_repository::get_latest_completed_run(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("No completed extraction run for document '{doc_id}'"),
        })?;

    // 4. Count pipeline DB items and relationships by type
    let items = pipeline_repository::get_items_for_run(&state.pipeline_pool, run_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;

    let rels = pipeline_repository::get_relationships_for_run(&state.pipeline_pool, run_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;

    let mut pipeline_items: HashMap<String, usize> = HashMap::new();
    for item in &items {
        *pipeline_items.entry(item.entity_type.clone()).or_insert(0) += 1;
    }
    let mut pipeline_rels: HashMap<String, usize> = HashMap::new();
    for rel in &rels {
        *pipeline_rels.entry(rel.relationship_type.clone()).or_insert(0) += 1;
    }

    let pipeline_db = PipelineCounts {
        total_items: items.len(),
        total_relationships: rels.len(),
        extraction_items: pipeline_items.clone(),
        extraction_relationships: pipeline_rels.clone(),
    };

    // 5. Count Neo4j nodes by label scoped to this document
    let neo4j_nodes = count_neo4j_nodes(&state, &doc_id).await?;
    let neo4j_rels = count_neo4j_relationships(&state, &doc_id).await?;
    let orphaned = find_orphaned_nodes(&state, &doc_id).await?;

    let neo4j_total_nodes: usize = neo4j_nodes.values().sum();
    let neo4j_total_rels: usize = neo4j_rels.values().sum();

    let neo4j = Neo4jCounts {
        nodes_by_label: neo4j_nodes.clone(),
        relationships_by_type: neo4j_rels.clone(),
        total_nodes: neo4j_total_nodes,
        total_relationships: neo4j_total_rels,
        orphaned_nodes: orphaned,
    };

    // 6. Count Qdrant points filtered by source_document
    let qdrant_count = qdrant_service::count_points_by_filter(
        &state.http_client, &state.config.qdrant_url, "source_document", &doc_id,
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Qdrant count error: {e}"),
    })?;

    let qdrant = QdrantCounts {
        total_points: qdrant_count,
        collection: "colossus_evidence".to_string(),
    };

    // 7. Run comparison checks
    let mut checks = Vec::new();

    // Check: Party count (pipeline "Party" == Neo4j Person + Organization)
    let pipeline_party = *pipeline_items.get("Party").unwrap_or(&0);
    let neo4j_person = *neo4j_nodes.get("Person").unwrap_or(&0);
    let neo4j_org = *neo4j_nodes.get("Organization").unwrap_or(&0);
    let neo4j_party_total = neo4j_person + neo4j_org;
    checks.push(CompletenessCheck {
        name: "party_count".into(),
        status: if pipeline_party == neo4j_party_total { "pass" } else { "fail" }.into(),
        expected: pipeline_party,
        actual: neo4j_party_total,
        message: format!(
            "Pipeline Party({pipeline_party}) vs Neo4j Person({neo4j_person})+Org({neo4j_org})"
        ),
    });

    // Check: FactualAllegation → ComplaintAllegation
    check_entity_count(
        &mut checks, &pipeline_items, &neo4j_nodes,
        "FactualAllegation", "ComplaintAllegation", "allegation_count",
    );

    // Check: DamagesClaim → Harm
    check_entity_count(
        &mut checks, &pipeline_items, &neo4j_nodes,
        "DamagesClaim", "Harm", "harm_count",
    );

    // Check: LegalCount → LegalCount
    check_entity_count(
        &mut checks, &pipeline_items, &neo4j_nodes,
        "LegalCount", "LegalCount", "legal_count_count",
    );

    // Check: Total nodes (pipeline items + 1 Document == Neo4j total)
    let expected_neo4j_nodes = items.len() + 1; // +1 for Document node
    checks.push(CompletenessCheck {
        name: "total_node_count".into(),
        status: if expected_neo4j_nodes == neo4j_total_nodes { "pass" } else { "fail" }.into(),
        expected: expected_neo4j_nodes,
        actual: neo4j_total_nodes,
        message: format!(
            "Pipeline items({}) + 1 Document vs Neo4j nodes({neo4j_total_nodes})",
            items.len()
        ),
    });

    // Check: Extraction relationships (exclude CONTAINED_IN from Neo4j)
    let neo4j_data_rels = neo4j_total_rels
        - neo4j_rels.get("CONTAINED_IN").copied().unwrap_or(0);
    checks.push(CompletenessCheck {
        name: "relationship_count".into(),
        status: if rels.len() == neo4j_data_rels { "pass" } else { "fail" }.into(),
        expected: rels.len(),
        actual: neo4j_data_rels,
        message: format!(
            "Pipeline rels({}) vs Neo4j rels({neo4j_data_rels}) (excl CONTAINED_IN)"
        , rels.len()),
    });

    // Check: Per-type relationship counts (STATED_BY, ABOUT, SUPPORTS)
    for rel_type in &["STATED_BY", "ABOUT", "SUPPORTS"] {
        let pipeline_count = *pipeline_rels.get(*rel_type).unwrap_or(&0);
        let neo4j_count = *neo4j_rels.get(*rel_type).unwrap_or(&0);
        checks.push(CompletenessCheck {
            name: format!("rel_{}", rel_type.to_lowercase()),
            status: if pipeline_count == neo4j_count { "pass" } else { "fail" }.into(),
            expected: pipeline_count,
            actual: neo4j_count,
            message: format!("{rel_type}: pipeline({pipeline_count}) vs neo4j({neo4j_count})"),
        });
    }

    // Check: Neo4j nodes vs Qdrant points
    checks.push(CompletenessCheck {
        name: "qdrant_point_count".into(),
        status: if neo4j_total_nodes == qdrant_count { "pass" } else { "fail" }.into(),
        expected: neo4j_total_nodes,
        actual: qdrant_count,
        message: format!("Neo4j nodes({neo4j_total_nodes}) vs Qdrant points({qdrant_count})"),
    });

    // Check: Orphaned nodes
    checks.push(CompletenessCheck {
        name: "orphaned_nodes".into(),
        status: if neo4j.orphaned_nodes.is_empty() { "pass" } else { "warn" }.into(),
        expected: 0,
        actual: neo4j.orphaned_nodes.len(),
        message: format!("{} orphaned nodes", neo4j.orphaned_nodes.len()),
    });

    // 8. Determine overall status
    let all_pass = checks.iter().all(|c| c.status != "fail");
    let overall_status = if all_pass { "pass" } else { "fail" };

    // 9. If all pass, update status → PUBLISHED
    let published = if all_pass && document.status != "PUBLISHED" {
        pipeline_repository::update_document_status(&state.pipeline_pool, &doc_id, "PUBLISHED")
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to update status: {e}"),
            })?;
        tracing::info!(doc_id = %doc_id, "Completeness passed — status → PUBLISHED");
        true
    } else {
        false
    };

    log_admin_action(
        &state.audit_repo, &user.username, "pipeline.document.completeness",
        Some("document"), Some(&doc_id),
        Some(serde_json::json!({
            "status": overall_status, "published": published,
            "neo4j_nodes": neo4j_total_nodes, "qdrant_points": qdrant_count,
        })),
    )
    .await;

    Ok(Json(CompletenessResponse {
        document_id: doc_id,
        status: overall_status.to_string(),
        pipeline_db,
        neo4j,
        qdrant,
        checks,
        published,
    }))
}
