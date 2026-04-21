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
        // Evidence: 1-hop join to get speaker name via STATED_BY relationship
        (
            "MATCH (e:Evidence)
             OPTIONAL MATCH (e)-[:STATED_BY]->(speaker)
             RETURN e.id AS id, 'Evidence' AS node_type,
                    e.title AS title,
                    e.verbatim_quote AS verbatim_quote,
                    e.significance AS significance,
                    e.page_number AS page_number,
                    e.document_id AS document_id,
                    e.statement_type AS statement_type,
                    e.statement_date AS statement_date,
                    e.exhibit_number AS exhibit_number,
                    e.kind AS kind,
                    COALESCE(speaker.name, '') AS stated_by",
            vec![
                "title",
                "verbatim_quote",
                "significance",
                "page_number",
                "document_id",
                "statement_type",
                "statement_date",
                "exhibit_number",
                "kind",
                "stated_by",
            ],
        ),
        (
            "MATCH (a:ComplaintAllegation)
             RETURN a.id AS id, 'ComplaintAllegation' AS node_type,
                    a.title AS title,
                    a.allegation AS allegation,
                    COALESCE(a.verbatim_quote, a.verbatim, '') AS verbatim_quote,
                    a.evidence_status AS evidence_status,
                    a.category AS category,
                    a.severity AS severity,
                    a.paragraph AS paragraph",
            vec![
                "title",
                "allegation",
                "verbatim_quote",
                "evidence_status",
                "category",
                "severity",
                "paragraph",
            ],
        ),
        (
            "MATCH (m:MotionClaim)
             RETURN m.id AS id, 'MotionClaim' AS node_type,
                    m.title AS title,
                    m.claim_text AS claim_text,
                    m.significance AS significance,
                    m.source_document_id AS source_document_id,
                    m.category AS category",
            vec![
                "title",
                "claim_text",
                "significance",
                "source_document_id",
                "category",
            ],
        ),
        (
            "MATCH (h:Harm)
             RETURN h.id AS id, 'Harm' AS node_type,
                    h.title AS title,
                    h.description AS description,
                    h.category AS category,
                    h.subcategory AS subcategory,
                    h.amount AS amount,
                    h.date AS date,
                    h.source_reference AS source_reference",
            vec![
                "title",
                "description",
                "category",
                "subcategory",
                "amount",
                "date",
                "source_reference",
            ],
        ),
        (
            "MATCH (d:Document)
             RETURN d.id AS id, 'Document' AS node_type,
                    d.title AS title,
                    d.document_type AS document_type,
                    d.date AS date,
                    d.page_count AS page_count,
                    d.file_path AS file_path",
            vec!["title", "document_type", "date", "page_count", "file_path"],
        ),
        (
            "MATCH (p:Person)
             RETURN p.id AS id, 'Person' AS node_type,
                    p.name AS name,
                    p.role AS role,
                    p.roles AS roles,
                    p.description AS description",
            vec!["name", "role", "roles", "description"],
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

/// Like `run_node_query` but accepts a single named parameter.
///
/// Used by `fetch_nodes_for_document` to pass the `$doc_id` parameter
/// into document-scoped Cypher queries. The existing `run_node_query`
/// is unchanged — the embed-all pipeline still uses it without params.
async fn run_node_query_with_param(
    graph: &Graph,
    cypher: &str,
    prop_keys: &[&str],
    param_name: &str,
    param_value: &str,
) -> Result<Vec<EmbeddableNode>, EmbeddingRepoError> {
    let mut nodes = Vec::new();
    let mut result = graph
        .execute(query(cypher).param(param_name, param_value))
        .await?;

    while let Some(row) = result.next().await? {
        let id: String = row.get("id").unwrap_or_default();
        let node_type: String = row.get("node_type").unwrap_or_default();

        if id.is_empty() {
            continue;
        }

        let mut properties = HashMap::new();
        for key in prop_keys {
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

/// Fetch embeddable nodes belonging to a specific document.
///
/// ## Why this is label-agnostic
///
/// Previously this function enumerated each entity label with a bespoke
/// Cypher query (ComplaintAllegation, Harm, Person, Organization,
/// LegalCount, Document). New labels introduced by future extraction
/// schemas (SwornStatement, DocumentReference, ...) would silently skip
/// the Index step and never reach Qdrant.
///
/// The fix: anchor on the `CONTAINED_IN` relationship that `Ingest` always
/// creates from every entity to its Document. One query matches all
/// current and future entity labels without code changes.
///
/// ## Shape of the RETURN
///
/// `labels(n)[0]` extracts the node's primary label as `node_type`. The
/// RETURN is a union of every string-valued property the
/// `build_embedding_text` builder reads for any existing type
/// (`title`, `name`, `verbatim_quote`, `description`, `role`,
/// `significance`, `allegation`, `claim_text`, `document_type`,
/// `source_document`). Missing properties for a given label become empty
/// strings and are omitted by `run_node_query_with_param`. The
/// `verbatim_quote`/`verbatim` COALESCE preserves backward compatibility
/// with older ComplaintAllegation writes.
pub async fn fetch_nodes_for_document(
    graph: &Graph,
    document_id: &str,
) -> Result<Vec<EmbeddableNode>, EmbeddingRepoError> {
    let mut all_nodes = Vec::new();

    // 1. The Document node itself (keyed on source_document_id rather
    //    than CONTAINED_IN — Documents aren't contained in themselves).
    let doc_cypher = "MATCH (d:Document)
         WHERE d.source_document_id = $doc_id
         RETURN d.id AS id, 'Document' AS node_type,
                d.title AS title,
                d.doc_type AS document_type,
                d.source_document_id AS source_document";
    let doc_prop_keys = vec!["title", "document_type", "source_document"];
    all_nodes.extend(
        run_node_query_with_param(graph, doc_cypher, &doc_prop_keys, "doc_id", document_id)
            .await?,
    );

    // 2. Every non-Document entity contained in that Document. Works for
    //    any entity label — current or future.
    let entity_cypher = "MATCH (n)-[:CONTAINED_IN]->(d:Document)
         WHERE d.source_document_id = $doc_id AND NOT n:Document
         RETURN n.id AS id,
                labels(n)[0] AS node_type,
                n.title AS title,
                n.name AS name,
                COALESCE(n.verbatim_quote, n.verbatim, '') AS verbatim_quote,
                n.description AS description,
                n.role AS role,
                n.significance AS significance,
                n.allegation AS allegation,
                n.claim_text AS claim_text,
                n.source_document AS source_document";
    let entity_prop_keys = vec![
        "title",
        "name",
        "verbatim_quote",
        "description",
        "role",
        "significance",
        "allegation",
        "claim_text",
        "source_document",
    ];
    all_nodes.extend(
        run_node_query_with_param(
            graph,
            entity_cypher,
            &entity_prop_keys,
            "doc_id",
            document_id,
        )
        .await?,
    );

    Ok(all_nodes)
}
