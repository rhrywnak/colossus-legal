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

use std::collections::{HashMap, HashSet};

use neo4rs::{query, Graph};

use crate::error::AppError;
use crate::repositories::pipeline_repository::ExtractionItemRecord;
use crate::state::AppState;

use super::ingest_helpers::slug;

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

/// Count unique party names per label (Person / Organization) across the
/// approved items of a run.
///
/// `create_party_nodes` MERGEs Party items on `slug(name)` in Neo4j, so 6
/// raw Party items with 3 unique names produce 3 Person nodes. Comparing
/// raw pipeline-item counts against Neo4j node counts therefore always
/// fails the completeness check for shared parties (bug 9a).
///
/// This helper reproduces the MERGE-dedup key (lowercased slug of
/// `party_name` → `full_name` → `"unknown"`) so the per-label comparison
/// is apples-to-apples.
///
/// Only labels that actually appear in `items` are present in the returned
/// map — callers merge it into `pipeline_items` to override those entries.
pub fn unique_party_counts(items: &[ExtractionItemRecord]) -> HashMap<String, usize> {
    let mut by_label: HashMap<String, HashSet<String>> = HashMap::new();
    for item in items {
        if item.entity_type != "Person" && item.entity_type != "Organization" {
            continue;
        }
        let props = &item.item_data["properties"];
        let name = props["party_name"]
            .as_str()
            .or_else(|| props["full_name"].as_str())
            .unwrap_or("unknown");
        by_label
            .entry(item.entity_type.clone())
            .or_default()
            .insert(slug(name));
    }
    by_label.into_iter().map(|(k, v)| (k, v.len())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(id: i32, entity_type: &str, party_name: &str) -> ExtractionItemRecord {
        ExtractionItemRecord {
            id,
            run_id: 1,
            document_id: "doc-test".to_string(),
            entity_type: entity_type.to_string(),
            item_data: serde_json::json!({
                "label": "test",
                "properties": { "party_name": party_name }
            }),
            verbatim_quote: None,
            grounding_status: None,
            grounded_page: None,
            review_status: "approved".to_string(),
            reviewed_by: None,
            reviewed_at: None,
            review_notes: None,
            graph_status: "written".to_string(),
        }
    }

    #[test]
    fn unique_party_counts_dedups_by_slugged_name() {
        // 6 Party items with 3 distinct names — case and whitespace differ
        // but slug() normalizes them so they collapse to 3 unique Persons.
        let items = vec![
            make_item(1, "Person", "Marie Awad"),
            make_item(2, "Person", "MARIE AWAD"),
            make_item(3, "Person", "John Smith"),
            make_item(4, "Person", "john smith"),
            make_item(5, "Person", "Jane Doe"),
            make_item(6, "Person", "jane  doe"),
        ];
        let counts = unique_party_counts(&items);
        assert_eq!(counts.get("Person").copied(), Some(3));
        assert!(counts.get("Organization").is_none());
    }

    #[test]
    fn unique_party_counts_separates_person_and_organization() {
        let items = vec![
            make_item(1, "Person", "Marie Awad"),
            make_item(2, "Organization", "Catholic Family Services"),
            make_item(3, "Organization", "catholic family services"),
        ];
        let counts = unique_party_counts(&items);
        assert_eq!(counts.get("Person").copied(), Some(1));
        assert_eq!(counts.get("Organization").copied(), Some(1));
    }

    #[test]
    fn unique_party_counts_ignores_non_party_entity_types() {
        let items = vec![
            make_item(1, "ComplaintAllegation", "not a party"),
            make_item(2, "LegalCount", "also not a party"),
        ];
        let counts = unique_party_counts(&items);
        assert!(counts.is_empty());
    }

    #[test]
    fn unique_party_counts_falls_back_to_full_name() {
        // Schemas may use `full_name` instead of `party_name`.
        let mut item = make_item(1, "Person", "ignored");
        item.item_data = serde_json::json!({
            "label": "test",
            "properties": { "full_name": "Marie Awad" }
        });
        let counts = unique_party_counts(std::slice::from_ref(&item));
        assert_eq!(counts.get("Person").copied(), Some(1));
    }
}
