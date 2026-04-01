//! Helper functions for the completeness check endpoint.
//!
//! Extracted from `completeness.rs` to keep it under 300 lines.
//! Contains Neo4j query helpers and comparison logic.

use std::collections::HashMap;

use neo4rs::query;

use crate::error::AppError;
use crate::state::AppState;

use super::completeness::CompletenessCheck;

/// Count Neo4j nodes by label, scoped to a document.
pub async fn count_neo4j_nodes(
    state: &AppState,
    doc_id: &str,
) -> Result<HashMap<String, usize>, AppError> {
    let cypher = "MATCH (n)
        WHERE n.source_document = $doc_id OR n.source_document_id = $doc_id
        RETURN labels(n)[0] AS label, count(n) AS count
        ORDER BY label";
    let mut result = state.graph.execute(query(cypher).param("doc_id", doc_id)).await
        .map_err(|e| AppError::Internal { message: format!("Neo4j node count error: {e}") })?;
    let mut counts = HashMap::new();
    while let Some(row) = result.next().await
        .map_err(|e| AppError::Internal { message: format!("Neo4j row error: {e}") })?
    {
        let label: String = row.get("label").unwrap_or_default();
        let count: i64 = row.get("count").unwrap_or(0);
        if !label.is_empty() {
            counts.insert(label, count as usize);
        }
    }
    Ok(counts)
}

/// Count Neo4j relationships by type, scoped to a document's outgoing nodes.
pub async fn count_neo4j_relationships(
    state: &AppState,
    doc_id: &str,
) -> Result<HashMap<String, usize>, AppError> {
    let cypher = "MATCH (a)-[r]->(b)
        WHERE a.source_document = $doc_id OR a.source_document_id = $doc_id
        RETURN type(r) AS rel_type, count(r) AS count
        ORDER BY rel_type";
    let mut result = state.graph.execute(query(cypher).param("doc_id", doc_id)).await
        .map_err(|e| AppError::Internal { message: format!("Neo4j rel count error: {e}") })?;
    let mut counts = HashMap::new();
    while let Some(row) = result.next().await
        .map_err(|e| AppError::Internal { message: format!("Neo4j row error: {e}") })?
    {
        let rel_type: String = row.get("rel_type").unwrap_or_default();
        let count: i64 = row.get("count").unwrap_or(0);
        if !rel_type.is_empty() {
            counts.insert(rel_type, count as usize);
        }
    }
    Ok(counts)
}

/// Find orphaned nodes (no relationships at all) scoped to a document.
pub async fn find_orphaned_nodes(
    state: &AppState,
    doc_id: &str,
) -> Result<Vec<String>, AppError> {
    let cypher = "MATCH (n)
        WHERE (n.source_document = $doc_id OR n.source_document_id = $doc_id)
          AND NOT (n)--()
        RETURN n.id AS id";
    let mut result = state.graph.execute(query(cypher).param("doc_id", doc_id)).await
        .map_err(|e| AppError::Internal { message: format!("Neo4j orphan query error: {e}") })?;
    let mut ids = Vec::new();
    while let Some(row) = result.next().await
        .map_err(|e| AppError::Internal { message: format!("Neo4j row error: {e}") })?
    {
        let id: String = row.get("id").unwrap_or_default();
        if !id.is_empty() {
            ids.push(id);
        }
    }
    Ok(ids)
}

/// Compare a pipeline entity type count against a Neo4j label count.
pub fn check_entity_count(
    checks: &mut Vec<CompletenessCheck>,
    pipeline: &HashMap<String, usize>,
    neo4j: &HashMap<String, usize>,
    pipeline_type: &str,
    neo4j_label: &str,
    check_name: &str,
) {
    let expected = *pipeline.get(pipeline_type).unwrap_or(&0);
    let actual = *neo4j.get(neo4j_label).unwrap_or(&0);
    checks.push(CompletenessCheck {
        name: check_name.into(),
        status: if expected == actual { "pass" } else { "fail" }.into(),
        expected,
        actual,
        message: format!("{pipeline_type}({expected}) vs {neo4j_label}({actual})"),
    });
}
