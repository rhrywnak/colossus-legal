//! GET /api/admin/pipeline/documents/:id/completeness — Completeness Check.
//! Cross-references pipeline DB, Neo4j, and Qdrant. Updates status to
//! PUBLISHED if everything matches.

use std::collections::HashMap;

use axum::{extract::Path, extract::State, Json};
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::pipeline::constants::QDRANT_DOCUMENT_ID_FIELD;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository::{self, steps};
use crate::services::qdrant_service;
use crate::state::AppState;

use super::completeness_helpers::{
    count_neo4j_nodes, count_neo4j_relationships, find_orphaned_nodes, unique_party_counts,
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

/// Core logic for completeness check — callable from handler AND process endpoint.
///
/// Cross-references pipeline DB, Neo4j, and Qdrant. Updates to PUBLISHED if pass.
/// Does NOT check document status — caller is responsible for validation.
pub(crate) async fn run_completeness(
    state: &AppState,
    doc_id: &str,
    username: &str,
) -> Result<CompletenessResponse, AppError> {
    let start = std::time::Instant::now();

    let step_id = steps::record_step_start(
        &state.pipeline_pool,
        doc_id,
        "completeness",
        username,
        &serde_json::json!({}),
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Step logging: {e}"),
    })?;

    // 1. Fetch document — must exist
    let document = pipeline_repository::get_document(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    // 2. Get latest completed extraction run
    let run_id = pipeline_repository::get_latest_completed_run(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("No completed extraction run for document '{doc_id}'"),
        })?;

    // 4. Count pipeline DB items and relationships by type.
    //    Only count APPROVED items — unapproved items are not in Neo4j,
    //    so including them would cause a count mismatch.
    let items =
        pipeline_repository::get_approved_items_for_document(&state.pipeline_pool, doc_id, run_id)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("DB error: {e}"),
            })?;

    let rels =
        pipeline_repository::get_approved_relationships_for_document(&state.pipeline_pool, run_id)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("DB error: {e}"),
            })?;

    let mut pipeline_items: HashMap<String, usize> = HashMap::new();
    for item in &items {
        *pipeline_items.entry(item.entity_type.clone()).or_insert(0) += 1;
    }

    // Bug 9a: `create_party_nodes` MERGEs parties by slug(name), so N
    // Party items with K unique names yield K Neo4j Person nodes, not N.
    // Replace the raw Person/Organization counts with unique-name counts
    // so the per-type check compares like-for-like.
    for (label, count) in unique_party_counts(&items) {
        pipeline_items.insert(label, count);
    }

    let mut pipeline_rels: HashMap<String, usize> = HashMap::new();
    for rel in &rels {
        *pipeline_rels
            .entry(rel.relationship_type.clone())
            .or_insert(0) += 1;
    }

    let pipeline_db = PipelineCounts {
        total_items: items.len(),
        total_relationships: rels.len(),
        extraction_items: pipeline_items.clone(),
        extraction_relationships: pipeline_rels.clone(),
    };

    // 5. Count Neo4j nodes by label scoped to this document
    let neo4j_nodes = count_neo4j_nodes(state, doc_id).await?;
    let neo4j_rels = count_neo4j_relationships(state, doc_id).await?;
    let orphaned = find_orphaned_nodes(state, doc_id).await?;

    let neo4j_total_nodes: usize = neo4j_nodes.values().sum();
    let neo4j_total_rels: usize = neo4j_rels.values().sum();

    let neo4j = Neo4jCounts {
        nodes_by_label: neo4j_nodes.clone(),
        relationships_by_type: neo4j_rels.clone(),
        total_nodes: neo4j_total_nodes,
        total_relationships: neo4j_total_rels,
        orphaned_nodes: orphaned,
    };

    // 6. Count Qdrant points filtered by document_id.
    //    Bug 9b: previously filtered by "source_document", which the Index
    //    step does write but Qdrant has no payload index for (see
    //    qdrant_service::ensure_collection). QDRANT_DOCUMENT_ID_FIELD
    //    ("document_id") IS indexed and is the authoritative filter key.
    let qdrant_count = qdrant_service::count_points_by_filter(
        &state.http_client,
        &state.config.qdrant_url,
        QDRANT_DOCUMENT_ID_FIELD,
        doc_id,
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Qdrant count error: {e}"),
    })?;

    let qdrant = QdrantCounts {
        total_points: qdrant_count,
        collection: "colossus_evidence".to_string(),
    };

    // 7. Run comparison checks using the pure comparison function.
    //
    // Since generic ingest (beta.41), pipeline entity_type == Neo4j label
    // directly. No translation mapping is needed — we compare counts
    // by grouping on the same string in both systems.
    let checks = compare_counts(&CompareInput {
        pipeline_items: &pipeline_items,
        neo4j_nodes: &neo4j_nodes,
        pipeline_rels: &pipeline_rels,
        neo4j_rels: &neo4j_rels,
        total_pipeline_items: items.len(),
        total_pipeline_rels: rels.len(),
        neo4j_total_nodes,
        neo4j_total_rels,
        qdrant_count,
        orphaned_node_count: neo4j.orphaned_nodes.len(),
    });

    // 8. Determine overall status
    let all_pass = checks.iter().all(|c| c.status != "fail");
    let overall_status = if all_pass { "pass" } else { "fail" };

    // 9. If all pass, update status → PUBLISHED
    let published = if all_pass && document.status != "PUBLISHED" {
        pipeline_repository::update_document_status(&state.pipeline_pool, doc_id, "PUBLISHED")
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
        &state.audit_repo,
        username,
        "pipeline.document.completeness",
        Some("document"),
        Some(doc_id),
        Some(serde_json::json!({
            "status": overall_status, "published": published,
            "neo4j_nodes": neo4j_total_nodes, "qdrant_points": qdrant_count,
        })),
    )
    .await;

    let checks_passed = checks.iter().filter(|c| c.status == "pass").count();
    let checks_failed = checks.iter().filter(|c| c.status == "fail").count();
    steps::record_step_complete(
        &state.pipeline_pool, step_id, start.elapsed().as_secs_f64(),
        &serde_json::json!({"checks_passed": checks_passed, "checks_failed": checks_failed, "published": published}),
    ).await.ok();

    Ok(CompletenessResponse {
        document_id: doc_id.to_string(),
        status: overall_status.to_string(),
        pipeline_db,
        neo4j,
        qdrant,
        checks,
        published,
    })
}

/// GET /api/admin/pipeline/documents/:id/completeness
///
/// HTTP handler — thin wrapper around `run_completeness`.
/// Checks admin auth and status guard, then delegates to core logic.
pub async fn completeness_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<CompletenessResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "GET completeness");

    // Status guard
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    if document.status != "INDEXED" && document.status != "PUBLISHED" {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot check completeness: status is '{}', expected 'INDEXED'",
                document.status
            ),
            details: serde_json::json!({ "status": document.status }),
        });
    }

    let result = run_completeness(&state, &doc_id, &user.username).await?;
    Ok(Json(result))
}

// ── Pure comparison logic (testable without DB) ────────────────

/// Input for the pure comparison function. Bundles all pre-fetched
/// counts from pipeline DB, Neo4j, and Qdrant so we can test without
/// any database access.
pub struct CompareInput<'a> {
    pub pipeline_items: &'a HashMap<String, usize>,
    pub neo4j_nodes: &'a HashMap<String, usize>,
    pub pipeline_rels: &'a HashMap<String, usize>,
    pub neo4j_rels: &'a HashMap<String, usize>,
    pub total_pipeline_items: usize,
    pub total_pipeline_rels: usize,
    pub neo4j_total_nodes: usize,
    pub neo4j_total_rels: usize,
    pub qdrant_count: usize,
    pub orphaned_node_count: usize,
}

/// Compare pipeline DB counts against Neo4j/Qdrant counts.
///
/// This is a pure function — no I/O, no database access. It takes
/// pre-fetched count maps and returns the list of completeness checks.
/// This design makes it easy to unit test with mock data.
///
/// ## How entity type comparison works (since beta.41)
///
/// The pipeline DB `entity_type` field stores the same string as the
/// Neo4j node label (e.g., "ComplaintAllegation", "Person"). No
/// translation mapping is needed. We dynamically iterate all entity
/// types found in either system and compare counts.
pub fn compare_counts(input: &CompareInput<'_>) -> Vec<CompletenessCheck> {
    let CompareInput {
        pipeline_items,
        neo4j_nodes,
        pipeline_rels,
        neo4j_rels,
        total_pipeline_items,
        total_pipeline_rels,
        neo4j_total_nodes,
        neo4j_total_rels,
        qdrant_count,
        orphaned_node_count,
    } = input;
    let mut checks = Vec::new();

    // Per-type entity counts: iterate all types from both sides.
    // This handles any entity type dynamically — no hardcoded names.
    let mut all_entity_types: Vec<&String> =
        pipeline_items.keys().chain(neo4j_nodes.keys()).collect();
    all_entity_types.sort();
    all_entity_types.dedup();

    // Skip "Document" from per-type checks — it's a structural node
    // created by ingest, not an extraction item.
    for entity_type in &all_entity_types {
        if entity_type.as_str() == "Document" {
            continue;
        }
        let expected = *pipeline_items.get(entity_type.as_str()).unwrap_or(&0);
        let actual = *neo4j_nodes.get(entity_type.as_str()).unwrap_or(&0);
        checks.push(CompletenessCheck {
            name: format!("entity_{}", entity_type.to_lowercase()),
            status: if expected == actual { "pass" } else { "fail" }.into(),
            expected,
            actual,
            message: format!("{entity_type}: pipeline({expected}) vs neo4j({actual})"),
        });
    }

    // Total nodes: pipeline items + 1 Document node == Neo4j total
    let expected_neo4j_nodes = *total_pipeline_items + 1;
    checks.push(CompletenessCheck {
        name: "total_node_count".into(),
        status: if expected_neo4j_nodes == *neo4j_total_nodes { "pass" } else { "fail" }.into(),
        expected: expected_neo4j_nodes,
        actual: *neo4j_total_nodes,
        message: format!(
            "Pipeline items({total_pipeline_items}) + 1 Document vs Neo4j nodes({neo4j_total_nodes})"
        ),
    });

    // Total relationships: exclude CONTAINED_IN (structural, not extraction data)
    let neo4j_data_rels = *neo4j_total_rels - neo4j_rels.get("CONTAINED_IN").copied().unwrap_or(0);
    checks.push(CompletenessCheck {
        name: "relationship_count".into(),
        status: if *total_pipeline_rels == neo4j_data_rels { "pass" } else { "fail" }.into(),
        expected: *total_pipeline_rels,
        actual: neo4j_data_rels,
        message: format!(
            "Pipeline rels({total_pipeline_rels}) vs Neo4j rels({neo4j_data_rels}) (excl CONTAINED_IN)"
        ),
    });

    // Per-type relationship counts: dynamic, like entity types
    let mut all_rel_types: Vec<&String> = pipeline_rels.keys().chain(neo4j_rels.keys()).collect();
    all_rel_types.sort();
    all_rel_types.dedup();

    for rel_type in &all_rel_types {
        // Skip CONTAINED_IN — it's structural (Document→entity), not extraction data
        if rel_type.as_str() == "CONTAINED_IN" {
            continue;
        }
        let expected = *pipeline_rels.get(rel_type.as_str()).unwrap_or(&0);
        let actual = *neo4j_rels.get(rel_type.as_str()).unwrap_or(&0);
        checks.push(CompletenessCheck {
            name: format!("rel_{}", rel_type.to_lowercase()),
            status: if expected == actual { "pass" } else { "fail" }.into(),
            expected,
            actual,
            message: format!("{rel_type}: pipeline({expected}) vs neo4j({actual})"),
        });
    }

    // Qdrant: Neo4j nodes should equal Qdrant points
    checks.push(CompletenessCheck {
        name: "qdrant_point_count".into(),
        status: if *neo4j_total_nodes == *qdrant_count {
            "pass"
        } else {
            "fail"
        }
        .into(),
        expected: *neo4j_total_nodes,
        actual: *qdrant_count,
        message: format!("Neo4j nodes({neo4j_total_nodes}) vs Qdrant points({qdrant_count})"),
    });

    // Orphaned nodes
    checks.push(CompletenessCheck {
        name: "orphaned_nodes".into(),
        status: if *orphaned_node_count == 0 {
            "pass"
        } else {
            "warn"
        }
        .into(),
        expected: 0,
        actual: *orphaned_node_count,
        message: format!("{orphaned_node_count} orphaned nodes"),
    });

    checks
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a HashMap from pairs.
    fn counts(pairs: &[(&str, usize)]) -> HashMap<String, usize> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn completeness_check_passes_with_v2_entity_types() {
        // Since beta.41, pipeline stores the same labels as Neo4j.
        let pipeline_items = counts(&[
            ("ComplaintAllegation", 83),
            ("Harm", 9),
            ("LegalCount", 4),
            ("Person", 9),
            ("Organization", 4),
        ]);
        let neo4j_nodes = counts(&[
            ("ComplaintAllegation", 83),
            ("Harm", 9),
            ("LegalCount", 4),
            ("Person", 9),
            ("Organization", 4),
            ("Document", 1),
        ]);
        let pipeline_rels = counts(&[("STATED_BY", 83), ("ABOUT", 9), ("SUPPORTS", 4)]);
        let neo4j_rels = counts(&[
            ("STATED_BY", 83),
            ("ABOUT", 9),
            ("SUPPORTS", 4),
            ("CONTAINED_IN", 109),
        ]);

        let total_items: usize = pipeline_items.values().sum(); // 109
        let total_rels: usize = pipeline_rels.values().sum(); // 96
        let neo4j_total_nodes: usize = neo4j_nodes.values().sum(); // 110
        let neo4j_total_rels: usize = neo4j_rels.values().sum(); // 205

        let checks = compare_counts(&CompareInput {
            pipeline_items: &pipeline_items,
            neo4j_nodes: &neo4j_nodes,
            pipeline_rels: &pipeline_rels,
            neo4j_rels: &neo4j_rels,
            total_pipeline_items: total_items,
            total_pipeline_rels: total_rels,
            neo4j_total_nodes,
            neo4j_total_rels,
            qdrant_count: neo4j_total_nodes, // qdrant == neo4j nodes
            orphaned_node_count: 0,
        });

        let failed: Vec<_> = checks.iter().filter(|c| c.status == "fail").collect();
        assert!(
            failed.is_empty(),
            "Expected all checks to pass, but these failed: {failed:?}"
        );
    }

    #[test]
    fn completeness_check_fails_on_count_mismatch() {
        let pipeline_items = counts(&[("ComplaintAllegation", 83)]);
        let neo4j_nodes = counts(&[
            ("ComplaintAllegation", 80), // 3 missing!
            ("Document", 1),
        ]);
        let empty = HashMap::new();

        let checks = compare_counts(&CompareInput {
            pipeline_items: &pipeline_items,
            neo4j_nodes: &neo4j_nodes,
            pipeline_rels: &empty,
            neo4j_rels: &empty,
            total_pipeline_items: 83,
            total_pipeline_rels: 0,
            neo4j_total_nodes: 81,
            neo4j_total_rels: 0,
            qdrant_count: 81,
            orphaned_node_count: 0,
        });

        let entity_check = checks
            .iter()
            .find(|c| c.name == "entity_complaintallegation")
            .expect("Should have entity_complaintallegation check");
        assert_eq!(entity_check.status, "fail");
        assert_eq!(entity_check.expected, 83);
        assert_eq!(entity_check.actual, 80);
    }

    #[test]
    fn completeness_check_passes_with_zero_counts() {
        // Empty pipeline + empty Neo4j is valid (no data to mismatch).
        // We need at least a Document node for total_node_count to pass.
        let empty = HashMap::new();
        let neo4j_nodes = counts(&[("Document", 1)]);

        let checks = compare_counts(&CompareInput {
            pipeline_items: &empty,
            neo4j_nodes: &neo4j_nodes,
            pipeline_rels: &empty,
            neo4j_rels: &empty,
            total_pipeline_items: 0,
            total_pipeline_rels: 0,
            neo4j_total_nodes: 1,
            neo4j_total_rels: 0,
            qdrant_count: 1,
            orphaned_node_count: 0,
        });

        let failed: Vec<_> = checks.iter().filter(|c| c.status == "fail").collect();
        assert!(
            failed.is_empty(),
            "Expected all checks to pass with zero counts, but these failed: {failed:?}"
        );
    }

    #[test]
    fn completeness_check_works_with_arbitrary_entity_types() {
        // Proves no hardcoded type dependency — works with any entity type name.
        let pipeline_items = counts(&[("CustomType", 10), ("AnotherType", 5)]);
        let neo4j_nodes = counts(&[("CustomType", 10), ("AnotherType", 5), ("Document", 1)]);
        let empty = HashMap::new();

        let checks = compare_counts(&CompareInput {
            pipeline_items: &pipeline_items,
            neo4j_nodes: &neo4j_nodes,
            pipeline_rels: &empty,
            neo4j_rels: &empty,
            total_pipeline_items: 15,
            total_pipeline_rels: 0,
            neo4j_total_nodes: 16,
            neo4j_total_rels: 0,
            qdrant_count: 16,
            orphaned_node_count: 0,
        });

        // Should have per-type checks for CustomType and AnotherType
        assert!(checks.iter().any(|c| c.name == "entity_customtype"));
        assert!(checks.iter().any(|c| c.name == "entity_anothertype"));

        let failed: Vec<_> = checks.iter().filter(|c| c.status == "fail").collect();
        assert!(
            failed.is_empty(),
            "Expected all checks to pass with arbitrary types, but these failed: {failed:?}"
        );
    }

    #[test]
    fn completeness_check_detects_extra_neo4j_type() {
        // Neo4j has a type that pipeline doesn't — should show 0 vs N.
        let pipeline_items = counts(&[("Person", 5)]);
        let neo4j_nodes = counts(&[
            ("Person", 5),
            ("Ghost", 3), // extra type not in pipeline
            ("Document", 1),
        ]);
        let empty = HashMap::new();

        let checks = compare_counts(&CompareInput {
            pipeline_items: &pipeline_items,
            neo4j_nodes: &neo4j_nodes,
            pipeline_rels: &empty,
            neo4j_rels: &empty,
            total_pipeline_items: 5,
            total_pipeline_rels: 0,
            neo4j_total_nodes: 9,
            neo4j_total_rels: 0,
            qdrant_count: 9,
            orphaned_node_count: 0,
        });

        let ghost_check = checks
            .iter()
            .find(|c| c.name == "entity_ghost")
            .expect("Should detect Ghost type from Neo4j side");
        assert_eq!(ghost_check.status, "fail");
        assert_eq!(ghost_check.expected, 0); // pipeline has 0
        assert_eq!(ghost_check.actual, 3); // neo4j has 3
    }
}
