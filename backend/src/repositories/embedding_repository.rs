//! Fetches all embeddable nodes from Neo4j for the embedding pipeline.
//!
//! Runs one Cypher query per node type (7 types, ~225 nodes total) and
//! collects results into a flat `Vec<EmbeddableNode>` using a flexible
//! `HashMap<String, String>` property bag.

use neo4rs::{query, Graph};
use std::collections::HashMap;

/// A node fetched from Neo4j, ready for embedding.
///
/// We use `HashMap<String, String>` instead of per-type structs because
/// the embedding pipeline only needs string properties for text building.
/// This keeps the repository generic across all 7 node types.
#[derive(Debug, Clone)]
pub struct EmbeddableNode {
    pub id: String,
    pub node_type: String,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingRepoError {
    #[error("Neo4j query error: {0}")]
    Neo4j(#[from] neo4rs::Error),

    #[error("Neo4j deserialization error: {0}")]
    Deserialization(#[from] neo4rs::DeError),
}

/// Fetch all embeddable nodes from Neo4j (7 node types).
///
/// Returns a single flat vector. Each node has its type tag and a property
/// bag containing whatever fields the Cypher query returned.
pub async fn fetch_all_embeddable_nodes(
    graph: &Graph,
) -> Result<Vec<EmbeddableNode>, EmbeddingRepoError> {
    let mut all_nodes = Vec::new();

    // Each tuple: (Cypher query, node_type label, list of property columns)
    let queries = vec![
        (
            "MATCH (e:Evidence)
             RETURN e.id AS id, 'Evidence' AS node_type,
                    e.title AS title,
                    e.verbatim_quote AS verbatim_quote,
                    e.significance AS significance,
                    e.page_number AS page_number,
                    e.document_id AS document_id",
            vec!["title", "verbatim_quote", "significance", "page_number", "document_id"],
        ),
        (
            "MATCH (a:ComplaintAllegation)
             RETURN a.id AS id, 'ComplaintAllegation' AS node_type,
                    a.title AS title,
                    a.allegation AS allegation,
                    a.verbatim AS verbatim",
            vec!["title", "allegation", "verbatim"],
        ),
        (
            "MATCH (m:MotionClaim)
             RETURN m.id AS id, 'MotionClaim' AS node_type,
                    m.title AS title,
                    m.claim_text AS claim_text,
                    m.significance AS significance",
            vec!["title", "claim_text", "significance"],
        ),
        (
            "MATCH (h:Harm)
             RETURN h.id AS id, 'Harm' AS node_type,
                    h.title AS title,
                    h.description AS description",
            vec!["title", "description"],
        ),
        (
            "MATCH (d:Document)
             RETURN d.id AS id, 'Document' AS node_type,
                    d.title AS title,
                    d.document_type AS document_type",
            vec!["title", "document_type"],
        ),
        (
            "MATCH (p:Person)
             RETURN p.id AS id, 'Person' AS node_type,
                    p.name AS name,
                    p.role AS role,
                    p.description AS description",
            vec!["name", "role", "description"],
        ),
        (
            "MATCH (o:Organization)
             RETURN o.id AS id, 'Organization' AS node_type,
                    o.name AS name,
                    o.role AS role,
                    o.description AS description",
            vec!["name", "role", "description"],
        ),
    ];

    for (cypher, prop_keys) in queries {
        let nodes = run_node_query(graph, cypher, &prop_keys).await?;
        all_nodes.extend(nodes);
    }

    Ok(all_nodes)
}

/// Execute a single Cypher query and extract nodes with the given property keys.
///
/// Every query must return `id` and `node_type` columns. The `prop_keys`
/// list tells us which additional columns to read into the properties map.
/// Missing or null values become empty strings — no panic.
async fn run_node_query(
    graph: &Graph,
    cypher: &str,
    prop_keys: &[&str],
) -> Result<Vec<EmbeddableNode>, EmbeddingRepoError> {
    let mut nodes = Vec::new();
    let mut result = graph.execute(query(cypher)).await?;

    while let Some(row) = result.next().await? {
        let id: String = row.get("id").unwrap_or_default();
        let node_type: String = row.get("node_type").unwrap_or_default();

        // Skip nodes without an ID (shouldn't happen, but be safe)
        if id.is_empty() {
            continue;
        }

        let mut properties = HashMap::new();
        for key in prop_keys {
            // Neo4j may return null for missing properties.
            // row.get::<String>() returns Err for nulls, so we default to "".
            let value: String = row.get(key).unwrap_or_default();
            if !value.is_empty() {
                properties.insert((*key).to_string(), value);
            }
        }

        nodes.push(EmbeddableNode {
            id,
            node_type,
            properties,
        });
    }

    Ok(nodes)
}
