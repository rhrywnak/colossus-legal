//! Reading the two stores and applying the twin merge, one cluster at a time.
//!
//! Everything that touches a store lives here; every DECISION lives in
//! [`super::plan`]. The Postgres side of one cluster is one transaction, counted
//! before the write and verified after it; a mismatch rolls that cluster back and
//! leaves it exactly as it was.
//!
//! ## The honest safety story, restated
//!
//! There is no cross-store transaction here and this module does not pretend
//! otherwise — same seam, same net as the re-key: the runbook takes database
//! backups immediately before `--apply`. What IS guaranteed is that the graph is
//! only touched after the Postgres side has been counted, written and verified,
//! so the window in which the two stores disagree is as short as this design
//! allows.

use neo4rs::Graph;
use sqlx::PgPool;

use super::graph::{collapse_in_graph, load_rows_with_edges};
use super::plan::{ClusterPlan, Disposition, TwinNode, TwinPlan};
use super::report::{ClusterProof, RunReport, TableProof};
use crate::oneshot::refs::{
    count_rows, repoint, table_proofs, EVIDENCE_CURATED_REFERENCES, EVIDENCE_REFERENCES,
};

/// Why the merge could not proceed.
#[derive(Debug, thiserror::Error)]
pub enum TwinMergeError {
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

    /// A merge target is already held by a node outside its cluster. Refused
    /// before anything is written.
    #[error(
        "the plan is unsafe: {count} merge target(s) are already held by a node \
         outside their cluster. Nothing was written. First: {first}"
    )]
    UnsafePlan { count: usize, first: String },

    /// A programming invariant was violated — this is a BUG in the tool, not a
    /// problem with the data.
    ///
    /// It exists so the defensive arms below have something honest to say.
    /// Reaching for `UnsafePlan` there would tell an operator to go looking for
    /// two nodes sharing an id, which is not what happened and would waste the
    /// only person who can actually fix it.
    #[error(
        "BUG in merge_evidence_twins, not a data problem: {what}. Nothing was \
         written. This needs a code fix, not an investigation of the corpus"
    )]
    InvariantViolated { what: String },
}

/// Read every Evidence node with its key fields and its edge fingerprints.
///
/// The edge fingerprints are collected in BOTH directions. Measured on DEV,
/// Evidence has no incoming edges at all — but "measured today" is not "true by
/// construction", and a merge that silently dropped an incoming edge because the
/// query never asked for one is exactly the class of bug this tool exists to
/// avoid.
pub async fn load_twin_nodes(
    graph: &Graph,
    pool: &PgPool,
) -> Result<Vec<TwinNode>, TwinMergeError> {
    let rows = load_rows_with_edges(graph).await?;
    let ids: Vec<String> = rows.iter().map(|(r, _)| r.current_id.clone()).collect();
    let curated = count_curated_per_node(pool, &ids).await?;

    Ok(rows
        .into_iter()
        .map(|(row, relationships)| TwinNode {
            curated_rows: curated_count(&curated, &row.current_id),
            row,
            relationships,
        })
        .collect())
}

/// How many curated rows point at one node.
fn curated_count(counts: &[(String, u64)], id: &str) -> u64 {
    counts
        .iter()
        .find(|(node_id, _)| node_id == id)
        .map(|(_, n)| *n)
        .unwrap_or(0)
}

/// Count, per node, how many CURATED rows reference it.
///
/// One query per curated column rather than one giant `UNION ALL`, so a column
/// added to the registry needs no SQL rewrite — and so a failure names which
/// table it came from.
async fn count_curated_per_node(
    pool: &PgPool,
    ids: &[String],
) -> Result<Vec<(String, u64)>, TwinMergeError> {
    const OP: &str = "count_curated_per_node";
    let mut totals: Vec<(String, u64)> = ids.iter().map(|id| (id.clone(), 0)).collect();

    for c in EVIDENCE_CURATED_REFERENCES {
        let sql = format!(
            "SELECT {} AS node_id, count(*) AS n FROM {} WHERE {} = ANY($1) GROUP BY 1",
            c.column, c.table, c.column
        );
        let rows: Vec<(String, i64)> = sqlx::query_as(&sql)
            .bind(ids)
            .fetch_all(pool)
            .await
            .map_err(|source| TwinMergeError::Postgres {
                operation: OP,
                source,
            })?;
        for (node_id, n) in rows {
            if let Some(entry) = totals.iter_mut().find(|(id, _)| id == &node_id) {
                entry.1 += n.max(0) as u64;
            }
        }
    }
    Ok(totals)
}

/// Plan the whole merge, refusing outright if the plan is unsafe.
pub fn plan_or_refuse(nodes: Vec<TwinNode>) -> Result<TwinPlan, TwinMergeError> {
    let all_ids: Vec<String> = nodes.iter().map(|n| n.id().to_string()).collect();
    let plan = TwinPlan::build(nodes);
    let conflicts = plan.target_conflicts(&all_ids);
    if !conflicts.is_empty() {
        let (target, holder) = &conflicts[0];
        return Err(TwinMergeError::UnsafePlan {
            count: conflicts.len(),
            first: format!("{target} already held by {holder}"),
        });
    }
    Ok(plan)
}

/// Execute the plan. With `apply == false` nothing is written.
pub async fn run(
    graph: &Graph,
    pool: &PgPool,
    plan: &TwinPlan,
    apply: bool,
) -> Result<RunReport, TwinMergeError> {
    let mut report = RunReport::from_plan(plan, apply);
    let totals = plan.totals();
    if !apply {
        tracing::info!(
            clusters = totals.clusters,
            to_merge = totals.clusters_to_merge,
            refused = totals.clusters_refused_curated + totals.clusters_refused_edges,
            "dry run — planning only, nothing will be written"
        );
        return Ok(report);
    }

    for cluster in plan.merges() {
        let proof = apply_cluster(graph, pool, cluster).await?;
        match &proof.aborted {
            Some(reason) => tracing::error!(
                cluster = %proof.key,
                reason = %reason,
                "cluster ABORTED and rolled back — its nodes are unchanged"
            ),
            None => tracing::info!(
                cluster = %proof.key,
                survivor = %proof.survivor,
                nodes_deleted = proof.nodes_deleted,
                rows_updated = proof.rows_updated(),
                "cluster merged"
            ),
        }
        report.clusters.push(proof);
    }
    Ok(report)
}

/// Open a transaction, count what points at the old ids, and repoint it.
///
/// Returns the OPEN transaction with the proofs, because the caller decides
/// whether to commit it — and that decision is made on the proofs plus what the
/// graph half reports afterwards.
///
/// ## Rust Learning: the `'a` tying the transaction to the pool
///
/// `sqlx::Transaction<'a, Postgres>` borrows the pool, and naming the lifetime
/// here is what lets the transaction escape this function while the compiler
/// still guarantees it cannot outlive the pool it came from. A transaction
/// dropped without a commit rolls back, which is the right default for a
/// function that hands one to a caller that might return early.
async fn repoint_postgres<'a>(
    pool: &'a PgPool,
    moves: &[(String, String)],
) -> Result<(sqlx::Transaction<'a, sqlx::Postgres>, Vec<TableProof>), TwinMergeError> {
    const OP: &str = "repoint_postgres";
    let old_ids: Vec<String> = moves.iter().map(|(old, _)| old.clone()).collect();

    let mut tx = pool
        .begin()
        .await
        .map_err(|source| TwinMergeError::Postgres {
            operation: OP,
            source,
        })?;
    let expected = count_rows(&mut tx, EVIDENCE_REFERENCES, &old_ids)
        .await
        .map_err(|source| TwinMergeError::Postgres {
            operation: "count_rows",
            source,
        })?;
    let updated = repoint(&mut tx, EVIDENCE_REFERENCES, moves)
        .await
        .map_err(|source| TwinMergeError::Postgres {
            operation: "repoint",
            source,
        })?;

    Ok((tx, table_proofs(EVIDENCE_REFERENCES, &expected, &updated)))
}

/// One cluster's unit of work: count, write, verify, then commit or roll back.
async fn apply_cluster(
    graph: &Graph,
    pool: &PgPool,
    cluster: &ClusterPlan,
) -> Result<ClusterProof, TwinMergeError> {
    const OP: &str = "apply_cluster";
    let Disposition::Merge {
        survivor,
        losers,
        target_id,
    } = &cluster.disposition
    else {
        // `run` only iterates merges, so this is unreachable today. It exists
        // because `let ... else` does NOT become a compile error when a new
        // `Disposition` variant is added — so a future variant would otherwise
        // fall silently into a write path.
        return Err(TwinMergeError::InvariantViolated {
            what: format!(
                "apply_cluster reached a non-Merge disposition for cluster {}",
                cluster.key
            ),
        });
    };

    // Every member's id moves to the target — the survivor's included, because
    // the target IS the stable-arm key and the survivor does not yet carry it.
    let moves: Vec<(String, String)> = cluster
        .members
        .iter()
        .map(|m| (m.id().to_string(), target_id.clone()))
        .collect();

    let (tx, tables) = repoint_postgres(pool, &moves).await?;
    let mut proof = ClusterProof {
        key: cluster.key.clone(),
        survivor: survivor.clone(),
        losers: losers.clone(),
        tables,
        nodes_deleted: 0,
        edges_deleted: 0,
        aborted: None,
    };

    if let Some(bad) = proof.tables.iter().find(|t| !t.is_sound()) {
        let reason = format!(
            "{}: expected {}, updated {}",
            bad.reference, bad.expected, bad.updated
        );
        tx.rollback()
            .await
            .map_err(|source| TwinMergeError::Postgres {
                operation: OP,
                source,
            })?;
        proof.aborted = Some(reason);
        return Ok(proof);
    }

    let (nodes_deleted, edges_deleted) =
        collapse_in_graph(graph, survivor, losers, target_id, cluster.members.len()).await?;
    proof.nodes_deleted = nodes_deleted;
    proof.edges_deleted = edges_deleted;

    tx.commit()
        .await
        .map_err(|source| TwinMergeError::Postgres {
            operation: OP,
            source,
        })?;
    Ok(proof)
}

#[cfg(test)]
#[path = "execute_tests.rs"]
mod tests;
