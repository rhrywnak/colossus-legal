//! Helper functions for the completeness check endpoint.
//!
//! Extracted from `completeness.rs` to keep it under 300 lines.
//! Contains Neo4j query helpers and comparison logic.
//!
//! Each Cypher helper exists in two flavors:
//! - `*_by_graph(&Graph, ...)` — the underlying query, returning raw
//!   `neo4rs::Error`. Used by the pipeline `Completeness` step which has
//!   an `AppContext` (not `AppState`).
//! - `*` — the HTTP-handler variant, a thin wrapper around `*_by_graph`
//!   that takes `&AppState` and wraps the error into `AppError::Internal`.

use std::collections::HashMap;

use neo4rs::{query, Graph};

use crate::error::AppError;
use crate::state::AppState;

// ─── Cypher constants ──────────────────────────────────────────────────

const CYPHER_COUNT_NODES: &str = "MATCH (n)
        WHERE n.source_document = $doc_id OR n.source_document_id = $doc_id
        RETURN labels(n)[0] AS label, count(n) AS count
        ORDER BY label";

const CYPHER_COUNT_RELATIONSHIPS: &str = "MATCH (a)-[r]->(b)
        WHERE a.source_document = $doc_id OR a.source_document_id = $doc_id
        RETURN type(r) AS rel_type, count(r) AS count
        ORDER BY rel_type";

const CYPHER_FIND_ORPHANED: &str = "MATCH (n)
        WHERE (n.source_document = $doc_id OR n.source_document_id = $doc_id)
          AND NOT (n)--()
        RETURN n.id AS id";

// ─── `_by_graph` variants — raw `neo4rs::Error`, used by pipeline step ───

/// Count Neo4j nodes by label, scoped to a document. Takes `&Graph`
/// directly — for the pipeline step, which has `AppContext`, not
/// `AppState`. Returns raw `neo4rs::Error`; callers wrap into their
/// own error type.
pub async fn count_neo4j_nodes_by_graph(
    graph: &Graph,
    doc_id: &str,
) -> Result<HashMap<String, usize>, neo4rs::Error> {
    let mut result = graph
        .execute(query(CYPHER_COUNT_NODES).param("doc_id", doc_id))
        .await?;
    let mut counts = HashMap::new();
    while let Some(row) = result.next().await? {
        let label: String = row.get("label").unwrap_or_default();
        let count: i64 = row.get("count").unwrap_or(0);
        if !label.is_empty() {
            counts.insert(label, count as usize);
        }
    }
    Ok(counts)
}

/// Count Neo4j relationships by type, scoped to a document's outgoing
/// nodes. `&Graph` variant — see `count_neo4j_nodes_by_graph`.
pub async fn count_neo4j_relationships_by_graph(
    graph: &Graph,
    doc_id: &str,
) -> Result<HashMap<String, usize>, neo4rs::Error> {
    let mut result = graph
        .execute(query(CYPHER_COUNT_RELATIONSHIPS).param("doc_id", doc_id))
        .await?;
    let mut counts = HashMap::new();
    while let Some(row) = result.next().await? {
        let rel_type: String = row.get("rel_type").unwrap_or_default();
        let count: i64 = row.get("count").unwrap_or(0);
        if !rel_type.is_empty() {
            counts.insert(rel_type, count as usize);
        }
    }
    Ok(counts)
}

/// Find orphaned nodes (no relationships at all) scoped to a document.
/// `&Graph` variant — see `count_neo4j_nodes_by_graph`.
pub async fn find_orphaned_nodes_by_graph(
    graph: &Graph,
    doc_id: &str,
) -> Result<Vec<String>, neo4rs::Error> {
    let mut result = graph
        .execute(query(CYPHER_FIND_ORPHANED).param("doc_id", doc_id))
        .await?;
    let mut ids = Vec::new();
    while let Some(row) = result.next().await? {
        let id: String = row.get("id").unwrap_or_default();
        if !id.is_empty() {
            ids.push(id);
        }
    }
    Ok(ids)
}

// ─── HTTP-handler variants — delegate to `_by_graph`, wrap into AppError ───

/// Count Neo4j nodes by label, scoped to a document.
pub async fn count_neo4j_nodes(
    state: &AppState,
    doc_id: &str,
) -> Result<HashMap<String, usize>, AppError> {
    count_neo4j_nodes_by_graph(&state.graph, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j node count error: {e}"),
        })
}

/// Count Neo4j relationships by type, scoped to a document's outgoing nodes.
pub async fn count_neo4j_relationships(
    state: &AppState,
    doc_id: &str,
) -> Result<HashMap<String, usize>, AppError> {
    count_neo4j_relationships_by_graph(&state.graph, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j rel count error: {e}"),
        })
}

/// Find orphaned nodes (no relationships at all) scoped to a document.
pub async fn find_orphaned_nodes(state: &AppState, doc_id: &str) -> Result<Vec<String>, AppError> {
    find_orphaned_nodes_by_graph(&state.graph, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j orphan query error: {e}"),
        })
}
