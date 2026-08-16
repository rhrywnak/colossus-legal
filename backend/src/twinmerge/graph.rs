//! The Cypher half of the twin merge: read the nodes and their edges, then
//! collapse a cluster.
//!
//! Split out of [`super::execute`] for size (Standing Rule 17). The seam is the
//! same one the party merge uses: this file is Cypher and its decoding,
//! `execute` is the order the statements run in and what a mismatch means.

use neo4rs::{query, Graph};

use super::execute::TwinMergeError;
use crate::models::document_status::ENTITY_EVIDENCE;
use crate::neo4j::schema;
use crate::rekey::plan::EvidenceRow;

/// The graph half of the load.
pub(super) async fn load_rows_with_edges(
    graph: &Graph,
) -> Result<Vec<(EvidenceRow, Vec<String>)>, TwinMergeError> {
    const OP: &str = "load_rows_with_edges";
    // `collect()` drops nulls, so a node with no edges yields an empty list
    // rather than a list holding one null.
    // Interpolated from `neo4j::schema`; a relationship type cannot be a bind
    // parameter, and a bare literal here would survive any rename.
    let cypher = format!(
        "MATCH (e)-[:{contained_in}]->(d) WHERE labels(e)[0] = $label \
         OPTIONAL MATCH (e)-[r]->(m) \
         WITH e, d, collect(DISTINCT type(r) + '->' + coalesce(m.id, '?')) AS outgoing \
         OPTIONAL MATCH (e)<-[r2]-(n) \
         WITH e, d, outgoing, \
              collect(DISTINCT '<-' + type(r2) + '-' + coalesce(n.id, '?')) AS incoming \
         RETURN e.id AS current_id, d.id AS doc_slug, e.page_number AS page, \
                e.verbatim_quote AS quote, e.question AS question, \
                outgoing + incoming AS edges \
         ORDER BY d.id, e.id",
        contained_in = schema::CONTAINED_IN
    );

    let mut stream = graph
        .execute(query(&cypher).param("label", ENTITY_EVIDENCE))
        .await
        .map_err(|source| TwinMergeError::Neo4jQuery {
            operation: OP,
            source,
        })?;

    let mut out = Vec::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(|source| TwinMergeError::Neo4jQuery {
            operation: OP,
            source,
        })?
    {
        let decode = |e: neo4rs::DeError| TwinMergeError::Neo4jDecode {
            operation: OP,
            source: e,
        };
        let mut edges: Vec<String> = row.get("edges").map_err(decode)?;
        // Sorted so two nodes' fingerprint lists compare as sets regardless of
        // the order the graph produced them.
        edges.sort();
        out.push((
            EvidenceRow {
                current_id: row.get("current_id").map_err(decode)?,
                doc_slug: row.get("doc_slug").map_err(decode)?,
                page: row.get("page").map_err(decode)?,
                verbatim_quote: row
                    .get::<Option<String>>("quote")
                    .map_err(decode)?
                    .unwrap_or_default(),
                question: row.get("question").map_err(decode)?,
            },
            edges,
        ));
    }
    Ok(out)
}

/// Delete the losers and move the survivor onto the cluster's key.
///
/// Returns `(nodes_deleted, edges_deleted)` as the graph itself reports them, not
/// as the plan predicted — the proof is only worth something if it counts what
/// happened.
pub(super) async fn collapse_in_graph(
    graph: &Graph,
    survivor: &str,
    losers: &[String],
    target_id: &str,
    occurrences: usize,
) -> Result<(u64, u64), TwinMergeError> {
    const OP: &str = "collapse_in_graph";
    let delete = "MATCH (l) WHERE l.id IN $losers AND labels(l)[0] = $label \
         OPTIONAL MATCH (l)-[r]-() \
         WITH collect(DISTINCT l) AS nodes, count(DISTINCT r) AS edges \
         FOREACH (n IN nodes | DETACH DELETE n) \
         RETURN size(nodes) AS nodes_deleted, edges AS edges_deleted";

    let mut stream = graph
        .execute(
            query(delete)
                .param("losers", losers.to_vec())
                .param("label", ENTITY_EVIDENCE),
        )
        .await
        .map_err(|source| TwinMergeError::Neo4jQuery {
            operation: OP,
            source,
        })?;

    let (nodes_deleted, edges_deleted) =
        match stream
            .next()
            .await
            .map_err(|source| TwinMergeError::Neo4jQuery {
                operation: OP,
                source,
            })? {
            Some(row) => {
                let decode = |e: neo4rs::DeError| TwinMergeError::Neo4jDecode {
                    operation: OP,
                    source: e,
                };
                let nodes: i64 = row.get("nodes_deleted").map_err(decode)?;
                let edges: i64 = row.get("edges_deleted").map_err(decode)?;
                (nodes.max(0) as u64, edges.max(0) as u64)
            }
            None => (0, 0),
        };

    // The survivor takes the key, and records that it stands for more than one
    // extraction. `occurrence_count` is provenance, not evidence: it says the
    // model emitted this statement N times, which is what the display layer's
    // "×2" already shows and what a future ingest-time dedupe will maintain.
    graph
        .run(
            query(
                "MATCH (s) WHERE s.id = $survivor AND labels(s)[0] = $label \
                 SET s.id = $target, s.occurrence_count = $occurrences",
            )
            .param("survivor", survivor)
            .param("target", target_id)
            .param("label", ENTITY_EVIDENCE)
            .param("occurrences", occurrences as i64),
        )
        .await
        .map_err(|source| TwinMergeError::Neo4jQuery {
            operation: "rekey_survivor",
            source,
        })?;

    Ok((nodes_deleted, edges_deleted))
}
