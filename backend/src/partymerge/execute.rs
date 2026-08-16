//! Reading the census and applying the party merge, one ruled cluster at a time.
//!
//! ## Why the edge types are a compiled list and not discovered at run time
//!
//! Moving a member's edges onto the survivor means CREATING an edge whose type
//! is only known at run time, and plain Cypher cannot do that — the type is part
//! of the query, not a parameter. The usual answer is an APOC procedure; this
//! module does not reach for one, because a merge that depends on a plugin fails
//! differently on a host where the plugin is absent, and "differently" is the
//! word that ends careers in a tool that deletes nodes.
//!
//! Instead the five measured types are compiled in, one statement is generated
//! per type, and [`preflight_edge_types`] REFUSES the whole run if the graph
//! holds an incident type the list does not know. A new relationship type is
//! therefore a loud failure before anything is written, never a silently dropped
//! edge.
//!
//! Measured on DEV 2026-08-15 — party-incident edges, with direction:
//! incoming `ABOUT` (1065), `STATED_BY` (514), `CHARACTERIZES` (44),
//! `SUFFERED_BY` (12); outgoing `CONTAINED_IN` (133).

use neo4rs::{query, Graph};
use sqlx::PgPool;

use super::census::PartyNode;
use super::graph_ops::merge_in_graph;
use super::plan::{ClusterPlan, Disposition, MergePlan};
use super::report::{ClusterProof, RunReport, TableProof};
use crate::models::document_status::{ENTITY_EVIDENCE, ENTITY_ORGANIZATION, ENTITY_PERSON};
use crate::neo4j::schema;
use crate::oneshot::refs::{count_rows, repoint, table_proofs, PARTY_REFERENCES};

/// Which way an edge points relative to the party node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Something points AT the party (`Evidence`-[:STATED_BY]->`Person`).
    Incoming,
    /// The party points at something (`Person`-[:CONTAINED_IN]->`Document`).
    Outgoing,
}

/// Every relationship type that touches a party node, with its direction.
///
// CONST: cannot come from config. A relationship TYPE is part of a Cypher query's
// text, not a bind parameter, so each type needs its own compiled statement — a
// runtime list could not produce one. The alternative is an APOC procedure, and a
// tool that DELETES nodes must not fail differently on a host where a plugin is
// absent. `preflight_edge_types` refuses the whole run rather than letting an
// unlisted type be silently deleted, which is what makes the compiled list safe.
pub const PARTY_EDGE_TYPES: &[(&str, Direction)] = &[
    (schema::ABOUT, Direction::Incoming),
    (schema::STATED_BY, Direction::Incoming),
    (schema::CHARACTERIZES, Direction::Incoming),
    (schema::SUFFERED_BY, Direction::Incoming),
    (schema::CONTAINED_IN, Direction::Outgoing),
];

/// Why the party merge could not proceed.
#[derive(Debug, thiserror::Error)]
pub enum PartyMergeError {
    #[error("Neo4j query failed during {operation}: {source}")]
    Neo4jQuery {
        operation: &'static str,
        #[source]
        source: neo4rs::Error,
    },

    #[error("failed to decode a Neo4j row during {operation}: {source}")]
    Neo4jDecode {
        operation: &'static str,
        #[source]
        source: neo4rs::DeError,
    },

    #[error("Postgres failed during {operation}: {source}")]
    Postgres {
        operation: &'static str,
        #[source]
        source: sqlx::Error,
    },

    #[error(
        "the graph holds party relationship type(s) this tool does not know how \
         to move: {types}. Nothing was written. Add them to PARTY_EDGE_TYPES \
         with the right direction, or the merge would delete them"
    )]
    UnknownEdgeTypes { types: String },

    /// A programming invariant was violated — this is a BUG in the tool, not a
    /// problem with the data or with the rulings file.
    ///
    /// Without it, the defensive arm below had to borrow `UnknownEdgeTypes`,
    /// whose message ends "add them to PARTY_EDGE_TYPES" — advice that would
    /// send an operator to edit the wrong file with the wrong value.
    #[error(
        "BUG in merge_parties, not a data problem: {what}. Nothing was written. \
         This needs a code fix, not a change to the rulings file"
    )]
    InvariantViolated { what: String },
}

/// Read every party node with the facts a ruling and a proof both need.
pub async fn load_census(graph: &Graph) -> Result<Vec<PartyNode>, PartyMergeError> {
    const OP: &str = "load_census";
    // The relationship type is part of the query TEXT, not a bind parameter, so
    // it is interpolated from `neo4j::schema` — the one place graph-schema names
    // live — rather than typed as a bare literal that no rename would find.
    let cypher = format!(
        "MATCH (p) WHERE labels(p)[0] IN [$person, $organization] \
         OPTIONAL MATCH (e)-[:{stated_by}]->(p) WHERE labels(e)[0] = $evidence \
         RETURN p.id AS id, labels(p)[0] AS label, \
                coalesce(p.party_name, p.name, p.id) AS display_name, \
                count(e) AS statements, \
                coalesce(p.source_documents, []) AS documents, \
                coalesce(p.aliases, []) AS aliases \
         ORDER BY p.id",
        stated_by = schema::STATED_BY
    );

    let mut stream = graph
        .execute(
            query(&cypher)
                .param("person", ENTITY_PERSON)
                .param("organization", ENTITY_ORGANIZATION)
                .param("evidence", ENTITY_EVIDENCE),
        )
        .await
        .map_err(|source| PartyMergeError::Neo4jQuery {
            operation: OP,
            source,
        })?;

    let mut parties = Vec::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(|source| PartyMergeError::Neo4jQuery {
            operation: OP,
            source,
        })?
    {
        let decode = |e: neo4rs::DeError| PartyMergeError::Neo4jDecode {
            operation: OP,
            source: e,
        };
        let statements: i64 = row.get("statements").map_err(decode)?;
        parties.push(PartyNode {
            id: row.get("id").map_err(decode)?,
            label: row.get("label").map_err(decode)?,
            display_name: row.get("display_name").map_err(decode)?,
            statement_count: statements.max(0) as u64,
            source_documents: row.get("documents").map_err(decode)?,
            aliases: row.get("aliases").map_err(decode)?,
        });
    }
    Ok(parties)
}

/// Refuse the run if any party-incident edge type is one this tool cannot move.
///
/// Runs before the first cluster, because discovering an unknown type half-way
/// through means some parties are merged and some are not.
pub async fn preflight_edge_types(graph: &Graph) -> Result<(), PartyMergeError> {
    const OP: &str = "preflight_edge_types";
    let cypher = "MATCH (p)-[r]-() WHERE labels(p)[0] IN [$person, $organization] \
         RETURN DISTINCT type(r) AS rel ORDER BY rel";

    let mut stream = graph
        .execute(
            query(cypher)
                .param("person", ENTITY_PERSON)
                .param("organization", ENTITY_ORGANIZATION),
        )
        .await
        .map_err(|source| PartyMergeError::Neo4jQuery {
            operation: OP,
            source,
        })?;

    let known: Vec<&str> = PARTY_EDGE_TYPES.iter().map(|(t, _)| *t).collect();
    let mut unknown = Vec::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(|source| PartyMergeError::Neo4jQuery {
            operation: OP,
            source,
        })?
    {
        let rel: String = row.get("rel").map_err(|e| PartyMergeError::Neo4jDecode {
            operation: OP,
            source: e,
        })?;
        if !known.contains(&rel.as_str()) {
            unknown.push(rel);
        }
    }
    if !unknown.is_empty() {
        return Err(PartyMergeError::UnknownEdgeTypes {
            types: unknown.join(", "),
        });
    }
    Ok(())
}

/// Execute the plan. With `apply == false` nothing is written.
pub async fn run(
    graph: &Graph,
    pool: &PgPool,
    plan: &MergePlan,
    apply: bool,
) -> Result<RunReport, PartyMergeError> {
    let mut report = RunReport::from_plan(plan, apply);
    let totals = plan.totals();
    if !apply {
        tracing::info!(
            ruled = totals.clusters_ruled,
            to_merge = totals.clusters_to_merge,
            skipped = totals.clusters_skipped,
            nodes_merging_in = totals.nodes_to_merge_in,
            "dry run — planning only, nothing will be written"
        );
        return Ok(report);
    }

    for cluster in plan.merges() {
        let proof = apply_cluster(graph, pool, cluster).await?;
        match &proof.aborted {
            Some(reason) => tracing::error!(
                cluster = %proof.label,
                reason = %reason,
                "cluster ABORTED and rolled back — its nodes are unchanged"
            ),
            None => tracing::info!(
                cluster = %proof.label,
                survivor = %proof.survivor,
                statements = proof.statements_after,
                edges_repointed = proof.edges_repointed,
                "cluster merged"
            ),
        }
        report.clusters.push(proof);
    }
    Ok(report)
}

/// One cluster's unit of work.
///
/// Both stores are transactional here, which the re-key could not manage: the
/// graph side runs in one `Txn` that is only committed after the statement count
/// proves the merge conserved everything. A failure rolls BOTH back.
async fn apply_cluster(
    graph: &Graph,
    pool: &PgPool,
    cluster: &ClusterPlan,
) -> Result<ClusterProof, PartyMergeError> {
    const OP: &str = "apply_cluster";
    let Disposition::Merge {
        survivor,
        members,
        expected_statements,
    } = &cluster.disposition
    else {
        // `run` only iterates merges, so this is unreachable today. It exists
        // because `let ... else` does NOT become a compile error when a new
        // `Disposition` variant is added — so a future variant would otherwise
        // fall silently into a write path.
        return Err(PartyMergeError::InvariantViolated {
            what: format!(
                "apply_cluster reached a non-Merge disposition for cluster {}",
                cluster.label
            ),
        });
    };

    let member_ids: Vec<String> = members.iter().map(|m| m.id.clone()).collect();

    let (pg, tables) = repoint_postgres(pool, &member_ids, &survivor.id).await?;
    let mut proof = ClusterProof {
        label: cluster.label.clone(),
        survivor: survivor.id.clone(),
        merged_in: member_ids.clone(),
        statements_expected: *expected_statements,
        statements_after: 0,
        nodes_deleted: 0,
        edges_repointed: 0,
        aliases_added: Vec::new(),
        tables,
        aborted: None,
    };

    let mut txn = graph
        .start_txn()
        .await
        .map_err(|source| PartyMergeError::Neo4jQuery {
            operation: "start_txn",
            source,
        })?;
    let outcome = merge_in_graph(&mut txn, survivor, &member_ids).await?;
    proof.edges_repointed = outcome.edges_repointed;
    proof.nodes_deleted = outcome.nodes_deleted;
    proof.aliases_added = outcome.aliases;
    proof.statements_after = outcome.statements_after;

    if let Some(reason) = proof.failure_reason() {
        roll_back_both(txn, pg).await?;
        proof.aborted = Some(reason);
        return Ok(proof);
    }

    txn.commit()
        .await
        .map_err(|source| PartyMergeError::Neo4jQuery {
            operation: "commit_graph",
            source,
        })?;
    pg.commit()
        .await
        .map_err(|source| PartyMergeError::Postgres {
            operation: OP,
            source,
        })?;
    Ok(proof)
}

/// Undo both halves of a cluster that failed its count proof.
///
/// Both, in this order, and both propagated: a rollback that itself fails leaves
/// the two stores in a state nobody planned, and it is the one thing here that
/// must never be swallowed. The graph goes first because it is the side that
/// deleted nodes.
async fn roll_back_both(
    txn: neo4rs::Txn,
    pg: sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), PartyMergeError> {
    txn.rollback()
        .await
        .map_err(|source| PartyMergeError::Neo4jQuery {
            operation: "rollback_graph",
            source,
        })?;
    pg.rollback()
        .await
        .map_err(|source| PartyMergeError::Postgres {
            operation: "rollback_postgres",
            source,
        })?;
    Ok(())
}

/// Open a transaction, count what points at the members, and repoint it.
///
/// Returns the OPEN transaction along with the proofs, because the caller
/// commits or rolls it back based on what the graph half then reports —
/// splitting the two would mean committing Postgres before knowing whether the
/// statements survived.
///
/// ## Rust Learning: returning an open `Transaction`
///
/// `sqlx::Transaction<'_, Postgres>` borrows the pool for its lifetime, and the
/// anonymous `'_` here ties the returned transaction to the `pool` argument — so
/// the compiler guarantees the caller cannot outlive the pool it came from. It
/// rolls back on drop if the caller never commits, which is the safe default for
/// exactly this shape.
async fn repoint_postgres<'a>(
    pool: &'a PgPool,
    member_ids: &[String],
    survivor_id: &str,
) -> Result<(sqlx::Transaction<'a, sqlx::Postgres>, Vec<TableProof>), PartyMergeError> {
    const OP: &str = "repoint_postgres";
    let moves: Vec<(String, String)> = member_ids
        .iter()
        .map(|id| (id.clone(), survivor_id.to_string()))
        .collect();

    let mut pg = pool
        .begin()
        .await
        .map_err(|source| PartyMergeError::Postgres {
            operation: OP,
            source,
        })?;
    let expected = count_rows(&mut pg, PARTY_REFERENCES, member_ids)
        .await
        .map_err(|source| PartyMergeError::Postgres {
            operation: "count_rows",
            source,
        })?;
    let updated = repoint(&mut pg, PARTY_REFERENCES, &moves)
        .await
        .map_err(|source| PartyMergeError::Postgres {
            operation: "repoint",
            source,
        })?;

    Ok((pg, table_proofs(PARTY_REFERENCES, &expected, &updated)))
}

#[cfg(test)]
#[path = "execute_tests.rs"]
mod tests;
