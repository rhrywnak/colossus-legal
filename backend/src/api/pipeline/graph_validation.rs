//! Post-ingest graph validation — checks that the Neo4j graph satisfies
//! structural requirements from the extraction schema.
//!
//! Runs after ingest, before transitioning to INDEXED/PUBLISHED.
//! Produces warnings and errors but does NOT block — the graph may be
//! incomplete and that's useful to know.

use std::path::Path;

use axum::{extract::Path as AxumPath, extract::State, Json};
use colossus_extract::{CompletenessRule, EntityCategory, ExtractionSchema};
use neo4rs::query;
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::models::document_status::{
    REL_CONTAINED_IN, STATUS_INDEXED, STATUS_INGESTED, STATUS_PUBLISHED,
};
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository::{self, steps};
use crate::state::AppState;

// ── Response DTOs ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct GraphValidationResult {
    pub document_id: String,
    pub checks_passed: Vec<String>,
    pub checks_failed: Vec<GraphValidationIssue>,
    pub warnings: Vec<GraphValidationIssue>,
}

#[derive(Debug, Serialize)]
pub struct GraphValidationIssue {
    pub check: String,
    pub severity: String,
    pub message: String,
    pub details: serde_json::Value,
}

// ── Handler ─────────────────────────────────────────────────────

/// POST /api/admin/pipeline/documents/:id/validate-graph
pub async fn validate_graph_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<Json<GraphValidationResult>, AppError> {
    require_admin(&user)?;
    let start = std::time::Instant::now();
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST validate-graph");

    let step_id = steps::record_step_start(
        &state.pipeline_pool,
        &doc_id,
        "validate_graph",
        &user.username,
        &serde_json::json!({}),
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Step logging: {e}"),
    })?;

    // 1. Verify document exists and is post-ingest
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    if !matches!(
        document.status.as_str(),
        STATUS_INGESTED | STATUS_INDEXED | STATUS_PUBLISHED
    ) {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot validate graph: status is '{}', expected {STATUS_INGESTED} or later",
                document.status
            ),
            details: serde_json::json!({"status": document.status}),
        });
    }

    // 2. Load schema
    let schema = load_schema(&state, &doc_id).await?;

    // 3. Run validation checks
    let result = run_validation(&state.graph, &schema, &doc_id).await?;

    let passed = result.checks_passed.len();
    let failed = result.checks_failed.len();
    let warnings = result.warnings.len();

    log_admin_action(
        &state.audit_repo,
        &user.username,
        "pipeline.document.validate_graph",
        Some("document"),
        Some(&doc_id),
        Some(serde_json::json!({
            "checks_passed": passed, "checks_failed": failed, "warnings": warnings,
        })),
    )
    .await;

    if let Err(e) = steps::record_step_complete(
        &state.pipeline_pool,
        step_id,
        start.elapsed().as_secs_f64(),
        &serde_json::json!({"passed": passed, "failed": failed, "warnings": warnings}),
    )
    .await
    {
        tracing::error!(
            document_id = %doc_id,
            step_id = step_id,
            error = %e,
            "Failed to record validate_graph step completion — audit trail gap"
        );
    }

    Ok(Json(result))
}

// ── Validation logic ────────────────────────────────────────────

async fn run_validation(
    graph: &neo4rs::Graph,
    schema: &ExtractionSchema,
    document_id: &str,
) -> Result<GraphValidationResult, AppError> {
    let mut passed = Vec::new();
    let mut failed = Vec::new();
    let mut warnings = Vec::new();

    // Check 1: Foundation entity counts
    for et in &schema.entity_types {
        if et.category == EntityCategory::Foundation && et.required {
            let label = &et.name;
            if !is_safe_label(label) {
                continue;
            }

            let count = count_nodes_by_label(graph, label, document_id).await?;
            let min = if et.min_count > 0 {
                et.min_count as usize
            } else {
                1
            };

            if count >= min {
                passed.push(format!(
                    "Foundation '{}': {} nodes (need {})",
                    label, count, min
                ));
            } else {
                failed.push(GraphValidationIssue {
                    check: format!("foundation_{}", label.to_lowercase()),
                    severity: "error".to_string(),
                    message: format!("Required foundation type '{}': found {}, need at least {}", label, count, min),
                    details: serde_json::json!({"entity_type": label, "found": count, "required": min}),
                });
            }
        }
    }

    // Check 2: Orphan check — nodes with no relationships except CONTAINED_IN
    let orphans = find_orphan_nodes(graph, document_id).await?;
    if orphans.is_empty() {
        passed.push("No orphan nodes found".to_string());
    } else {
        warnings.push(GraphValidationIssue {
            check: "orphan_nodes".to_string(),
            severity: "warning".to_string(),
            message: format!(
                "{} nodes have no relationships other than CONTAINED_IN",
                orphans.len()
            ),
            details: serde_json::json!({"orphan_ids": orphans}),
        });
    }

    // Check 3: Document linked — all nodes have CONTAINED_IN to Document
    let unlinked = find_unlinked_nodes(graph, document_id).await?;
    if unlinked.is_empty() {
        passed.push("All nodes linked to Document via CONTAINED_IN".to_string());
    } else {
        failed.push(GraphValidationIssue {
            check: "document_linked".to_string(),
            severity: "error".to_string(),
            message: format!(
                "{} nodes missing CONTAINED_IN relationship to Document",
                unlinked.len()
            ),
            details: serde_json::json!({"unlinked_ids": unlinked}),
        });
    }

    // Check 4: Completeness rule — RelationshipExists checks
    for rule in &schema.completeness_rules {
        if let CompletenessRule::RelationshipExists {
            from,
            relationship,
            to,
            min_percentage,
            message,
        } = rule
        {
            if !is_safe_label(from) || !is_safe_label(to) || !is_safe_label(relationship) {
                continue;
            }

            let percentage =
                compute_relationship_coverage(graph, document_id, from, relationship, to).await?;

            if percentage >= *min_percentage {
                passed.push(format!(
                    "{}: {}% (need {}%)",
                    message, percentage, min_percentage
                ));
            } else {
                warnings.push(GraphValidationIssue {
                    check: format!(
                        "rel_coverage_{}_{}",
                        from.to_lowercase(),
                        relationship.to_lowercase()
                    ),
                    severity: "warning".to_string(),
                    message: format!("{} ({}%, need {}%)", message, percentage, min_percentage),
                    details: serde_json::json!({
                        "from": from, "relationship": relationship, "to": to,
                        "actual_percentage": percentage, "required_percentage": min_percentage,
                    }),
                });
            }
        }
    }

    Ok(GraphValidationResult {
        document_id: document_id.to_string(),
        checks_passed: passed,
        checks_failed: failed,
        warnings,
    })
}

/// Validate that an evidence document's entities reference
/// parties and counts that exist in the graph.
/// Called after ingesting non-complaint (evidence) documents.
#[allow(dead_code)]
pub async fn validate_cross_document(
    _graph: &neo4rs::Graph,
    _schema: &ExtractionSchema,
    document_id: &str,
) -> Result<GraphValidationResult, AppError> {
    // TODO: Implement in F7
    Ok(GraphValidationResult {
        document_id: document_id.to_string(),
        checks_passed: vec!["Cross-document validation not yet implemented".to_string()],
        checks_failed: vec![],
        warnings: vec![],
    })
}

// ── Neo4j query helpers ─────────────────────────────────────────

/// Validate a label name for safe Cypher interpolation.
fn is_safe_label(label: &str) -> bool {
    !label.is_empty() && label.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Count nodes by label scoped to a document.
async fn count_nodes_by_label(
    graph: &neo4rs::Graph,
    label: &str,
    document_id: &str,
) -> Result<usize, AppError> {
    let cypher =
        format!("MATCH (n:{label}) WHERE n.source_document = $doc_id RETURN count(n) AS cnt");
    let mut result = graph
        .execute(query(&cypher).param("doc_id", document_id))
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j count {label}: {e}"),
        })?;
    let count: i64 = match result.next().await {
        Ok(Some(row)) => row.get("cnt").unwrap_or(0),
        Ok(None) => {
            tracing::warn!(
                document_id = %document_id,
                label = %label,
                "Neo4j count query returned no result row"
            );
            0
        }
        Err(e) => {
            tracing::error!(
                document_id = %document_id,
                label = %label,
                error = %e,
                "Failed to read Neo4j count — graph validation result is unreliable"
            );
            0
        }
    };
    Ok(count as usize)
}

/// Find nodes with only CONTAINED_IN relationships (orphans).
async fn find_orphan_nodes(
    graph: &neo4rs::Graph,
    document_id: &str,
) -> Result<Vec<String>, AppError> {
    let cypher = format!(
        "MATCH (n) \
         WHERE (n.source_document = $doc_id OR n.source_document_id = $doc_id) \
           AND NOT n:Document \
         WITH n \
         WHERE ALL(r IN [(n)-[rel]-() | type(rel)] WHERE r = '{REL_CONTAINED_IN}') \
         RETURN n.id AS id"
    );
    let mut result = graph
        .execute(query(&cypher).param("doc_id", document_id))
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j orphan query: {e}"),
        })?;
    let mut ids = Vec::new();
    while let Some(row) = result.next().await.map_err(|e| AppError::Internal {
        message: format!("Neo4j row: {e}"),
    })? {
        let id: String = row.get("id").unwrap_or_default();
        if !id.is_empty() {
            ids.push(id);
        }
    }
    Ok(ids)
}

/// Find nodes missing CONTAINED_IN relationship to Document.
async fn find_unlinked_nodes(
    graph: &neo4rs::Graph,
    document_id: &str,
) -> Result<Vec<String>, AppError> {
    let cypher = format!(
        "MATCH (n) \
         WHERE (n.source_document = $doc_id OR n.source_document_id = $doc_id) \
           AND NOT n:Document \
           AND NOT (n)-[:{REL_CONTAINED_IN}]->(:Document) \
         RETURN n.id AS id"
    );
    let mut result = graph
        .execute(query(&cypher).param("doc_id", document_id))
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j unlinked query: {e}"),
        })?;
    let mut ids = Vec::new();
    while let Some(row) = result.next().await.map_err(|e| AppError::Internal {
        message: format!("Neo4j row: {e}"),
    })? {
        let id: String = row.get("id").unwrap_or_default();
        if !id.is_empty() {
            ids.push(id);
        }
    }
    Ok(ids)
}

/// Compute the percentage of `from` nodes that have a relationship to a `to` node.
async fn compute_relationship_coverage(
    graph: &neo4rs::Graph,
    document_id: &str,
    from_label: &str,
    rel_type: &str,
    to_label: &str,
) -> Result<u32, AppError> {
    // Count total `from` nodes
    let total = count_nodes_by_label(graph, from_label, document_id).await?;
    if total == 0 {
        return Ok(100);
    } // No from nodes → vacuously true

    // Count `from` nodes that have at least one relationship to a `to` node
    let cypher = format!(
        "MATCH (a:{from_label})-[:{rel_type}]->(b:{to_label}) \
         WHERE a.source_document = $doc_id \
         RETURN count(DISTINCT a) AS linked"
    );
    let mut result = graph
        .execute(query(&cypher).param("doc_id", document_id))
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j coverage query {from_label}-{rel_type}->{to_label}: {e}"),
        })?;
    let linked: i64 = match result.next().await {
        Ok(Some(row)) => row.get("linked").unwrap_or(0),
        Ok(None) => {
            tracing::warn!(
                document_id = %document_id,
                from_label = %from_label,
                rel_type = %rel_type,
                to_label = %to_label,
                "Neo4j coverage query returned no result row"
            );
            0
        }
        Err(e) => {
            tracing::error!(
                document_id = %document_id,
                from_label = %from_label,
                rel_type = %rel_type,
                to_label = %to_label,
                error = %e,
                "Failed to read Neo4j coverage count — graph validation result is unreliable"
            );
            0
        }
    };

    Ok((linked as f64 / total as f64 * 100.0) as u32)
}

// ── Schema loader ───────────────────────────────────────────────

async fn load_schema(state: &AppState, doc_id: &str) -> Result<ExtractionSchema, AppError> {
    let pipe_config = pipeline_repository::get_pipeline_config(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("No pipeline config for '{doc_id}'"),
        })?;

    let schema_path = format!(
        "{}/{}",
        state.config.extraction_schema_dir, pipe_config.schema_file
    );
    ExtractionSchema::from_file(Path::new(&schema_path)).map_err(|e| AppError::Internal {
        message: format!("Failed to load schema '{}': {e}", pipe_config.schema_file),
    })
}
