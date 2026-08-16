//! The Cypher half of the party merge: repoint, union, delete, recount.
//!
//! Split out of [`super::execute`] for size (Standing Rule 17), but the seam is a
//! real one: everything here is a single Cypher statement and its decoding, and
//! everything in `execute` is the ORDER those statements run in and what is done
//! with a mismatch. A reader chasing "what Cypher does this actually run?" reads
//! this file and nothing else.
//!
//! ## Why the edge types are a compiled list and not discovered at run time
//!
//! Moving a member's edges onto the survivor means CREATING an edge whose type is
//! only known at run time, and plain Cypher cannot do that — the type is part of
//! the query, not a parameter. The usual answer is an APOC procedure; this module
//! does not reach for one, because a merge that depends on a plugin fails
//! differently on a host where the plugin is absent, and "differently" is the
//! word that ends careers in a tool that deletes nodes.
//!
//! Instead the five measured types are compiled into
//! [`super::execute::PARTY_EDGE_TYPES`], one statement is generated per type, and
//! `preflight_edge_types` REFUSES the whole run if the graph holds an incident
//! type that list does not know.

use neo4rs::{query, Txn};

use super::census::PartyNode;
use super::execute::{Direction, PartyMergeError, PARTY_EDGE_TYPES};
use crate::models::document_status::ENTITY_EVIDENCE;
use crate::neo4j::schema;

/// What the graph half of one cluster did.
pub(super) struct GraphOutcome {
    pub(super) edges_repointed: u64,
    pub(super) nodes_deleted: u64,
    pub(super) aliases: Vec<String>,
    pub(super) statements_after: u64,
}

/// Repoint edges, union the provenance, delete the members, recount.
pub(super) async fn merge_in_graph(
    txn: &mut Txn,
    survivor: &PartyNode,
    member_ids: &[String],
) -> Result<GraphOutcome, PartyMergeError> {
    let label = survivor.label.as_str();
    let mut edges_repointed = 0u64;
    for (rel_type, direction) in PARTY_EDGE_TYPES {
        edges_repointed +=
            repoint_edges(txn, label, &survivor.id, member_ids, rel_type, *direction).await?;
    }
    let aliases = union_provenance(txn, label, &survivor.id, member_ids).await?;
    let nodes_deleted = delete_members(txn, label, member_ids).await?;
    let statements_after = count_statements(txn, label, &survivor.id).await?;

    Ok(GraphOutcome {
        edges_repointed,
        nodes_deleted,
        aliases,
        statements_after,
    })
}

/// Move one relationship type from the members onto the survivor.
///
/// `MERGE` gives union semantics — an edge the survivor already has is not
/// duplicated — and `ON CREATE SET nr = properties(r)` carries the original's
/// `extraction_run_id` / `source_document_id` / `created_at` across. When the
/// survivor already has the edge, its own provenance is kept: both are true
/// records of the same fact, and inventing a merge rule for two provenances
/// would be the tool making something up.
async fn repoint_edges(
    txn: &mut Txn,
    label: &str,
    survivor: &str,
    member_ids: &[String],
    rel_type: &str,
    direction: Direction,
) -> Result<u64, PartyMergeError> {
    // `rel_type` and `label` come from compiled-in constants and the census's
    // own label value, never from operator input.
    let pattern = match direction {
        Direction::Incoming => format!(
            "MATCH (x)-[r:{rel_type}]->(m) MERGE (x)-[nr:{rel_type}]->(s) \
             ON CREATE SET nr = properties(r) DELETE r"
        ),
        Direction::Outgoing => format!(
            "MATCH (m)-[r:{rel_type}]->(x) MERGE (s)-[nr:{rel_type}]->(x) \
             ON CREATE SET nr = properties(r) DELETE r"
        ),
    };
    let cypher = format!(
        "MATCH (s) WHERE s.id = $survivor AND labels(s)[0] = $label \
         MATCH (m) WHERE m.id IN $members AND labels(m)[0] = $label \
         WITH s, m {pattern} RETURN count(*) AS moved"
    );

    let mut stream = txn
        .execute(
            query(&cypher)
                .param("survivor", survivor)
                .param("members", member_ids.to_vec())
                .param("label", label),
        )
        .await
        .map_err(|source| PartyMergeError::Neo4jQuery {
            operation: "repoint_edges",
            source,
        })?;
    read_count(&mut stream, txn, "moved", "repoint_edges").await
}

/// Union `source_documents` and `aliases` onto the survivor, and record every
/// merged node's own name as an alias.
///
/// Domain note: the merged names are the whole reason a reader can still find
/// the judge by searching "Tighe" after the node is called "Karen A. Tighe".
/// Losing them would make the merge itself a small act of forgetting.
async fn union_provenance(
    txn: &mut Txn,
    label: &str,
    survivor: &str,
    member_ids: &[String],
) -> Result<Vec<String>, PartyMergeError> {
    // The trailing `+ [null]` keeps the row alive when a list is empty —
    // `UNWIND []` produces no rows at all — and `collect` drops the null.
    let cypher = "MATCH (s) WHERE s.id = $survivor AND labels(s)[0] = $label \
         OPTIONAL MATCH (m) WHERE m.id IN $members AND labels(m)[0] = $label \
         WITH s, collect(m) AS ms \
         WITH s, ms, \
              coalesce(s.source_documents, []) + \
              reduce(a = [], m IN ms | a + coalesce(m.source_documents, [])) AS docs, \
              coalesce(s.aliases, []) + \
              reduce(a = [], m IN ms | \
                     a + coalesce(m.aliases, []) + [coalesce(m.party_name, m.name)]) AS als \
         UNWIND docs + [null] AS d \
         WITH s, als, collect(DISTINCT d) AS udocs \
         UNWIND als + [null] AS a \
         WITH s, udocs, collect(DISTINCT a) AS uals \
         SET s.source_documents = udocs, s.aliases = uals \
         RETURN uals AS aliases";

    let mut stream = txn
        .execute(
            query(cypher)
                .param("survivor", survivor)
                .param("members", member_ids.to_vec())
                .param("label", label),
        )
        .await
        .map_err(|source| PartyMergeError::Neo4jQuery {
            operation: "union_provenance",
            source,
        })?;

    match stream
        .next(txn.handle())
        .await
        .map_err(|source| PartyMergeError::Neo4jQuery {
            operation: "union_provenance",
            source,
        })? {
        Some(row) => row
            .get("aliases")
            .map_err(|e| PartyMergeError::Neo4jDecode {
                operation: "union_provenance",
                source: e,
            }),
        None => Ok(Vec::new()),
    }
}

/// Delete the merged nodes.
///
/// Plain `DELETE`, never `DETACH DELETE`. By this point every edge has been
/// repointed, so a member still holding one means an edge type nobody moved —
/// and `DELETE` turns that into a loud Neo4j error rather than the silent
/// deletion `DETACH` would perform.
async fn delete_members(
    txn: &mut Txn,
    label: &str,
    member_ids: &[String],
) -> Result<u64, PartyMergeError> {
    let cypher = "MATCH (m) WHERE m.id IN $members AND labels(m)[0] = $label \
         WITH collect(m) AS ms, count(m) AS n \
         FOREACH (x IN ms | DELETE x) \
         RETURN n AS deleted";

    let mut stream = txn
        .execute(
            query(cypher)
                .param("members", member_ids.to_vec())
                .param("label", label),
        )
        .await
        .map_err(|source| PartyMergeError::Neo4jQuery {
            operation: "delete_members",
            source,
        })?;
    read_count(&mut stream, txn, "deleted", "delete_members").await
}

/// Sworn statements attributed to the survivor, counted inside the transaction.
async fn count_statements(
    txn: &mut Txn,
    label: &str,
    survivor: &str,
) -> Result<u64, PartyMergeError> {
    // Interpolated from `neo4j::schema`, like every other relationship type in
    // this module; a bare literal would survive a rename and count zero.
    let cypher = format!(
        "MATCH (e)-[:{stated_by}]->(s) \
         WHERE s.id = $survivor AND labels(s)[0] = $label AND labels(e)[0] = $evidence \
         RETURN count(e) AS n",
        stated_by = schema::STATED_BY
    );

    let mut stream = txn
        .execute(
            query(&cypher)
                .param("survivor", survivor)
                .param("label", label)
                .param("evidence", ENTITY_EVIDENCE),
        )
        .await
        .map_err(|source| PartyMergeError::Neo4jQuery {
            operation: "count_statements",
            source,
        })?;
    read_count(&mut stream, txn, "n", "count_statements").await
}

/// Read one `count(*)`-shaped column off a transaction stream.
async fn read_count(
    stream: &mut neo4rs::RowStream,
    txn: &mut Txn,
    column: &str,
    operation: &'static str,
) -> Result<u64, PartyMergeError> {
    match stream
        .next(txn.handle())
        .await
        .map_err(|source| PartyMergeError::Neo4jQuery { operation, source })?
    {
        Some(row) => {
            let n: i64 = row.get(column).map_err(|e| PartyMergeError::Neo4jDecode {
                operation,
                source: e,
            })?;
            Ok(n.max(0) as u64)
        }
        // No row means the pattern matched nothing, which is zero — and the
        // caller's count proof is what turns an unexpected zero into an abort.
        None => Ok(0),
    }
}
