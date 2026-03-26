//! Neo4j query helpers for the document evidence/content endpoint.
//!
//! Extracted from `admin_document_evidence.rs` to keep that module
//! under the 300-line limit. Contains the UNION ALL Cypher query and
//! the functions that execute it.

use neo4rs::query;

use crate::error::AppError;

// ── Data transfer struct ────────────────────────────────────────

/// Intermediate struct to hold content data from Neo4j before merging
/// with PostgreSQL audit data. Works for all node types returned by
/// the UNION ALL query.
pub(crate) struct ContentNode {
    pub id: String,
    pub node_type: String,
    pub title: Option<String>,
    pub verbatim_quote: Option<String>,
    pub page_number: Option<String>,
    pub kind: Option<String>,
    pub weight: Option<String>,
    pub speaker: Option<String>,
}

// ── Document metadata ───────────────────────────────────────────

/// Fetch document title and source_type from Neo4j.
///
/// Returns `(title, source_type)`. The `source_type` tells the frontend
/// whether text highlighting is available for this document.
pub(crate) async fn fetch_document_meta(
    graph: &neo4rs::Graph,
    doc_id: &str,
) -> Result<(String, Option<String>), AppError> {
    let cypher = "MATCH (d:Document {id: $doc_id}) \
                  RETURN d.title AS title, d.source_type AS source_type";
    let mut result = graph
        .execute(query(cypher).param("doc_id", doc_id))
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j query failed: {e}"),
        })?;

    if let Some(row) = result.next().await.map_err(|e| AppError::Internal {
        message: format!("Neo4j row fetch failed: {e}"),
    })? {
        let title = row
            .get::<String>("title")
            .unwrap_or_else(|_| doc_id.to_string());
        let source_type: Option<String> = row.get("source_type").ok();
        Ok((title, source_type))
    } else {
        Err(AppError::NotFound {
            message: format!("Document not found: {doc_id}"),
        })
    }
}

// ── Content query ───────────────────────────────────────────────

/// UNION ALL Cypher query that retrieves all content node types linked
/// to a document. Each branch returns the same column names/types.
///
/// ## Rust Learning: UNION ALL type requirements
///
/// Neo4j UNION ALL requires every branch to return columns with matching
/// types. Integer fields like `page_number` and `paragraph` are wrapped
/// in `toString()` so all branches return strings. Fields that don't
/// exist on a particular node type are returned as `null`.
const CONTENT_QUERY: &str = "\
MATCH (n:Evidence)-[:CONTAINED_IN]->(d:Document {id: $doc_id})
OPTIONAL MATCH (n)-[:STATED_BY]->(p:Person)
RETURN n.id AS id, 'Evidence' AS node_type,
       n.title AS title, n.verbatim_quote AS verbatim_quote,
       toString(n.page_number) AS page_number, n.kind AS kind,
       n.weight AS weight, p.name AS speaker

UNION ALL

MATCH (n:ComplaintAllegation)-[:CONTAINED_IN]->(d:Document {id: $doc_id})
RETURN n.id AS id, 'ComplaintAllegation' AS node_type,
       n.allegation AS title, n.verbatim AS verbatim_quote,
       toString(n.paragraph) AS page_number, n.evidence_status AS kind,
       null AS weight, null AS speaker

UNION ALL

MATCH (n:LegalCount)-[:CONTAINED_IN]->(d:Document {id: $doc_id})
RETURN n.id AS id, 'LegalCount' AS node_type,
       n.title AS title, n.description AS verbatim_quote,
       null AS page_number, null AS kind,
       null AS weight, null AS speaker

UNION ALL

MATCH (n:Harm)-[:CONTAINED_IN]->(d:Document {id: $doc_id})
RETURN n.id AS id, 'Harm' AS node_type,
       n.title AS title, n.description AS verbatim_quote,
       null AS page_number, null AS kind,
       toString(n.amount) AS weight, null AS speaker

UNION ALL

MATCH (n:MotionClaim)-[:APPEARS_IN]->(d:Document {id: $doc_id})
RETURN n.id AS id, 'MotionClaim' AS node_type,
       n.title AS title, n.description AS verbatim_quote,
       toString(n.page_number) AS page_number, n.significance AS kind,
       null AS weight, null AS speaker";

/// Fetch all content nodes linked to a document via UNION ALL query.
///
/// ## Rust Learning: `row.get("field").ok()` pattern
///
/// In neo4rs, `row.get("field")` returns `Result<T, Error>`. When a
/// field is NULL in Neo4j, `.get()` returns `Err`, not `Ok(None)`.
/// Using `.ok()` converts `Result<String, _>` to `Option<String>`,
/// so a NULL field becomes `None` rather than an error.
pub(crate) async fn fetch_content_for_document(
    graph: &neo4rs::Graph,
    doc_id: &str,
) -> Result<Vec<ContentNode>, AppError> {
    let mut result = graph
        .execute(query(CONTENT_QUERY).param("doc_id", doc_id))
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j query failed: {e}"),
        })?;

    let mut nodes = Vec::new();
    while let Some(row) = result.next().await.map_err(|e| AppError::Internal {
        message: format!("Neo4j row fetch failed: {e}"),
    })? {
        nodes.push(ContentNode {
            id: row.get("id").unwrap_or_default(),
            node_type: row.get("node_type").unwrap_or_default(),
            title: row.get("title").ok(),
            verbatim_quote: row.get("verbatim_quote").ok(),
            page_number: row.get("page_number").ok(),
            kind: row.get("kind").ok(),
            weight: row.get("weight").ok(),
            speaker: row.get("speaker").ok(),
        });
    }

    Ok(nodes)
}
