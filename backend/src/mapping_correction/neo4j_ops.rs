//! Neo4j-side primitives for the B2 mapping-correction tool.
//!
//! These are deliberately separate from the ingest helper
//! [`crate::api::pipeline::ingest_helpers::write_cross_tier_relationship`]:
//! that helper stamps `asserted_by_document` on every edge it writes, which is
//! exactly the property the pipeline's per-document reconciliation keys its
//! delete on. An authored correction must NOT carry that property, or a later
//! complaint reprocess would silently delete it. So the correction tool owns
//! its own bespoke Cypher that never sets `asserted_by_document`.
//!
//! All edges use the single relationship-type constant
//! [`crate::neo4j::schema::BEARS_ON`]; the provenance value is bound as a
//! parameter from [`pipeline_repository::PROVENANCE_AUTHORED`] so the graph and
//! Postgres agree on one spelling.

use neo4rs::{query, Graph};

use super::MappingError;
use crate::neo4j::schema::BEARS_ON;
use crate::repositories::pipeline_repository::PROVENANCE_AUTHORED;

/// Does a node with this `id` exist? `label` gates the match (`Some("Element")`
/// for the target; `None` for the Allegation source, mirroring the ingest write
/// path which matches the source purely by `id`). The label, when present, is a
/// code-defined constant — never operator input — so interpolating it carries
/// no injection risk (Cypher cannot parameterize a label).
pub async fn node_exists(
    graph: &Graph,
    id: &str,
    label: Option<&str>,
) -> Result<bool, MappingError> {
    const OP: &str = "node_exists";
    let pattern = match label {
        Some(l) => format!("(n:{l} {{id: $id}})"),
        None => "(n {id: $id})".to_string(),
    };
    let cypher = format!("MATCH {pattern} RETURN count(n) AS c");
    let mut stream = graph
        .execute(query(&cypher).param("id", id))
        .await
        .map_err(|source| MappingError::Neo4j { operation: OP, source })?;
    match stream
        .next()
        .await
        .map_err(|source| MappingError::Neo4j { operation: OP, source })?
    {
        Some(row) => {
            let c: i64 = row
                .get("c")
                .map_err(|source| MappingError::Neo4jDecode { operation: OP, source })?;
            Ok(c > 0)
        }
        // A `count(...)` query always returns one row; no row means the server
        // returned an empty result, which we treat as "does not exist".
        None => Ok(false),
    }
}

/// MERGE an authored `BEARS_ON` edge. Deliberately omits `asserted_by_document`
/// so reconciliation never deletes it; stamps `provenance = 'authored'` on the
/// edge so the graph is self-describing. Idempotent: a second call MATCHes the
/// existing edge and re-sets the same provenance. The caller MUST have verified
/// both nodes exist (a MERGE whose MATCH finds no node is a silent no-op).
pub async fn merge_authored_edge(
    graph: &Graph,
    from_id: &str,
    to_id: &str,
) -> Result<(), MappingError> {
    const OP: &str = "merge_authored_edge";
    let cypher = format!(
        "MATCH (a {{id: $from_id}}) \
         MATCH (e:Element {{id: $to_id}}) \
         MERGE (a)-[r:{BEARS_ON}]->(e) \
         ON CREATE SET r.provenance = $prov, r.created_at = datetime() \
         ON MATCH  SET r.provenance = $prov, r.updated_at = datetime()"
    );
    graph
        .run(
            query(&cypher)
                .param("from_id", from_id)
                .param("to_id", to_id)
                .param("prov", PROVENANCE_AUTHORED),
        )
        .await
        .map_err(|source| MappingError::Neo4j { operation: OP, source })
}

/// Delete the `BEARS_ON` edge for one pair. Returns the number of edges removed
/// (0 if it was already absent — the delete is idempotent). Matches the source
/// purely by `id` and the target as `:Element`, the same shape as the writer.
pub async fn delete_edge(graph: &Graph, from_id: &str, to_id: &str) -> Result<u64, MappingError> {
    const OP: &str = "delete_edge";
    let cypher = format!(
        "MATCH (a {{id: $from_id}})-[r:{BEARS_ON}]->(e:Element {{id: $to_id}}) \
         DELETE r RETURN count(r) AS deleted"
    );
    let mut stream = graph
        .execute(query(&cypher).param("from_id", from_id).param("to_id", to_id))
        .await
        .map_err(|source| MappingError::Neo4j { operation: OP, source })?;
    match stream
        .next()
        .await
        .map_err(|source| MappingError::Neo4j { operation: OP, source })?
    {
        Some(row) => {
            let deleted: i64 = row
                .get("deleted")
                .map_err(|source| MappingError::Neo4jDecode { operation: OP, source })?;
            // count() is never negative; the cast is saturating-safe in range.
            Ok(deleted.max(0) as u64)
        }
        None => Ok(0),
    }
}

/// Promote an existing `BEARS_ON` edge to authored: set `provenance = 'authored'`
/// and strip ALL THREE extraction-origin properties so the edge no longer
/// falsely claims a source document or extraction run. Idempotent — REMOVE of an
/// absent property is a no-op. A pair with no edge yields zero updates (the
/// caller has already confirmed the Postgres row exists; a missing graph edge is
/// reported by the orchestration, not here).
pub async fn promote_edge(graph: &Graph, from_id: &str, to_id: &str) -> Result<(), MappingError> {
    const OP: &str = "promote_edge";
    let cypher = format!(
        "MATCH (a {{id: $from_id}})-[r:{BEARS_ON}]->(e:Element {{id: $to_id}}) \
         SET r.provenance = $prov \
         REMOVE r.asserted_by_document, r.source_document_id, r.extraction_run_id"
    );
    graph
        .run(
            query(&cypher)
                .param("from_id", from_id)
                .param("to_id", to_id)
                .param("prov", PROVENANCE_AUTHORED),
        )
        .await
        .map_err(|source| MappingError::Neo4j { operation: OP, source })
}
