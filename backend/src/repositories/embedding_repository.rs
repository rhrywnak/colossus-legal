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
// STRUCTURAL: Cypher is wire vocabulary for the Neo4j protocol, not a
// deployment-variable setting. Held at module scope, with its property-key list
// beside it, so `embedding_repository_tests` asserts against THE TEXT THAT RUNS
// rather than a copy — the two have to agree or the property is silently absent
// from the map and the builder that reads it becomes dead code.
const Q_ALL_EVIDENCE: &str = "MATCH (e:Evidence)
             OPTIONAL MATCH (e)-[:STATED_BY]->(speaker)
             RETURN e.id AS id, 'Evidence' AS node_type,
                    e.title AS title,
                    e.question AS question,
                    e.verbatim_quote AS verbatim_quote,
                    e.significance AS significance,
                    e.page_number AS page_number,
                    e.document_id AS document_id,
                    e.statement_type AS statement_type,
                    e.statement_date AS statement_date,
                    e.exhibit_number AS exhibit_number,
                    e.kind AS kind,
                    COALESCE(speaker.name, '') AS stated_by";

/// The columns [`Q_ALL_EVIDENCE`] returns that become properties.
///
/// `question` is the interrogatory or request for admission the card answers.
/// Without it `build_embedding_text` never sees one and its Request/Answer
/// composition is dead code — 367 cards, 99 of them with an answer-only quote,
/// would keep embedding as "Admitted." and nothing would fail.
const EVIDENCE_PROP_KEYS: [&str; 11] = [
    "title",
    "question",
    "verbatim_quote",
    "significance",
    "page_number",
    "document_id",
    "statement_type",
    "statement_date",
    "exhibit_number",
    "kind",
    "stated_by",
];

/// The label-agnostic per-document read. `question` is Evidence-only in
/// practice; any other label returns null for it and `run_node_query_with_param`
/// drops empty values before they reach the map, so no other node type sees it.
const Q_DOCUMENT_ENTITIES: &str = "MATCH (n)-[:CONTAINED_IN]->(d:Document)
         WHERE d.source_document_id = $doc_id AND NOT n:Document
         RETURN n.id AS id,
                labels(n)[0] AS node_type,
                n.title AS title,
                n.name AS name,
                n.question AS question,
                COALESCE(n.verbatim_quote, n.verbatim, '') AS verbatim_quote,
                n.description AS description,
                n.role AS role,
                n.significance AS significance,
                n.allegation AS allegation,
                n.claim_text AS claim_text,
                n.source_document AS source_document";

/// The columns [`Q_DOCUMENT_ENTITIES`] returns that become properties.
const ENTITY_PROP_KEYS: [&str; 10] = [
    "title",
    "name",
    "question",
    "verbatim_quote",
    "description",
    "role",
    "significance",
    "allegation",
    "claim_text",
    "source_document",
];

pub async fn fetch_all_embeddable_nodes(
    graph: &Graph,
) -> Result<Vec<EmbeddableNode>, EmbeddingRepoError> {
    let mut all_nodes = Vec::new();

    // Each tuple: (Cypher query, node_type label, list of property columns)
    let queries = vec![
        // Evidence: 1-hop join to get speaker name via STATED_BY relationship
        (Q_ALL_EVIDENCE, EVIDENCE_PROP_KEYS.to_vec()),
        (
            // v5.1 migration:
            //   - Label `:ComplaintAllegation` → `:Allegation`.
            //   - Embedding namespace tag `'ComplaintAllegation'` →
            //     `'Allegation'`. This invalidates the existing v4
            //     namespace — every existing Allegation embedding must be
            //     re-embedded after deploy. Roman authorized: pure v5.1
            //     stance, no back-compat with the v4 namespace name.
            //   - Property `a.allegation` (v4 prose) → `a.summary` (v5.1).
            //   - Property `a.paragraph` → `a.paragraph_number`.
            //   - Property `a.evidence_status` dropped (v5.1 has no
            //     equivalent); returned as `NULL`.
            //   - `a.title`, `a.category`, `a.severity`, `a.verbatim_quote`
            //     are stable.
            "MATCH (a:Allegation)
             RETURN a.id AS id, 'Allegation' AS node_type,
                    a.title AS title,
                    a.summary AS allegation,
                    COALESCE(a.verbatim_quote, a.verbatim, '') AS verbatim_quote,
                    NULL AS evidence_status,
                    a.category AS category,
                    a.severity AS severity,
                    a.paragraph_number AS paragraph",
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
                    d.doc_type AS document_type,
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
/// (`title`, `name`, `question`, `verbatim_quote`, `description`, `role`,
/// `significance`, `allegation`, `claim_text`, `document_type`,
/// `source_document`). `question` was added 2026-09-04: the Evidence arm of
/// the builder composes it with the quote, and a column that is not SELECTed
/// here is a property the builder can never see. Missing properties for a given label become empty
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
        run_node_query_with_param(graph, doc_cypher, &doc_prop_keys, "doc_id", document_id).await?,
    );

    // 2. Every non-Document entity contained in that Document. Works for
    //    any entity label — current or future.
    all_nodes.extend(
        run_node_query_with_param(
            graph,
            Q_DOCUMENT_ENTITIES,
            &ENTITY_PROP_KEYS,
            "doc_id",
            document_id,
        )
        .await?,
    );

    Ok(all_nodes)
}

#[cfg(test)]
#[path = "embedding_repository_tests.rs"]
mod tests;
