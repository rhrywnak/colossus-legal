//! Reading the graph and applying the re-key, one document at a time.
//!
//! Everything that touches a store lives here; every DECISION lives in
//! [`super::plan`]. The split is what makes the plan unit-testable and keeps this
//! file to the mechanics: read, count, write, verify, commit or abort.
//!
//! ## The eight referencing columns
//!
//! Seven tables, eight columns — `scenario_human_facts` carries two. They are
//! listed once, in [`REFERENCING_COLUMNS`], and every count and every update
//! walks that one list. A column added to the schema and forgotten here would
//! leave dangling rows the count proof could not see, which is why the list is a
//! constant with a test pinning its membership rather than eight hand-written
//! statements.

use std::collections::HashMap;

use neo4rs::{query, Graph};
use sqlx::PgPool;

use super::plan::{Disposition, EvidenceRow, PlannedNode, RekeyPlan};
use super::report::{DocumentProof, RunReport, TableProof};
use crate::models::document_status::ENTITY_EVIDENCE;

/// Every `(table, column)` that stores an Evidence graph id.
///
/// Measured 2026-08-14: 947 rows across these eight, referencing 148 distinct
/// Evidence ids. `scenario_human_facts` appears twice — an anchor and an answer
/// are two different references and both must move.
pub const REFERENCING_COLUMNS: &[(&str, &str)] = &[
    ("scenario_fact_refs", "graph_node_id"),
    ("scenario_human_facts", "anchor_graph_node_id"),
    ("scenario_human_facts", "answers_graph_node_id"),
    ("evidence_allegation_links", "graph_node_id"),
    ("evidence_allegation_link_events", "graph_node_id"),
    ("scenario_ruling_anchors", "graph_node_id"),
    ("scenario_candidate_ordinals", "graph_node_id"),
    ("scan_run_verdicts", "graph_node_id"),
];

/// Why the re-key could not proceed.
#[derive(Debug, thiserror::Error)]
pub enum RekeyError {
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

    /// The plan itself is unsafe — two nodes would end the run sharing an id.
    /// Refused before anything is written.
    #[error(
        "the plan is unsafe: {count} id(s) would be claimed by more than one node. \
         Nothing was written. First: {first}"
    )]
    UnsafePlan { count: usize, first: String },
}

/// Read every Evidence node with the fields the key is built from.
///
/// The speaker is deliberately NOT read: it is not in the key, and reading it
/// here would invite a future edit to slip it in.
pub async fn load_evidence_rows(graph: &Graph) -> Result<Vec<EvidenceRow>, RekeyError> {
    const OP: &str = "load_evidence_rows";
    let cypher = "MATCH (e)-[:CONTAINED_IN]->(d) \
         WHERE labels(e)[0] = $evidence_label \
         RETURN e.id AS current_id, d.id AS doc_slug, e.page_number AS page, \
                e.verbatim_quote AS quote, e.question AS question \
         ORDER BY d.id, e.id";

    let mut stream = graph
        .execute(query(cypher).param("evidence_label", ENTITY_EVIDENCE))
        .await
        .map_err(|source| RekeyError::Neo4jQuery {
            operation: OP,
            source,
        })?;

    let mut rows = Vec::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(|source| RekeyError::Neo4jQuery {
            operation: OP,
            source,
        })?
    {
        let decode = |e: neo4rs::DeError| RekeyError::Neo4jDecode {
            operation: OP,
            source: e,
        };
        rows.push(EvidenceRow {
            current_id: row.get("current_id").map_err(decode)?,
            doc_slug: row.get("doc_slug").map_err(decode)?,
            page: row.get("page").map_err(decode)?,
            verbatim_quote: row
                .get::<Option<String>>("quote")
                .map_err(decode)?
                .unwrap_or_default(),
            question: row.get("question").map_err(decode)?,
        });
    }
    Ok(rows)
}

/// Plan the whole re-key, refusing outright if the plan is unsafe.
///
/// The safety check runs BEFORE any document is touched, because an id collision
/// discovered halfway through would leave the corpus half re-keyed with two
/// statements sharing one node.
pub fn plan_or_refuse(rows: Vec<EvidenceRow>) -> Result<RekeyPlan, RekeyError> {
    let plan = RekeyPlan::build(rows);
    let conflicts = plan.target_conflicts();
    if !conflicts.is_empty() {
        let (id, holders) = &conflicts[0];
        return Err(RekeyError::UnsafePlan {
            count: conflicts.len(),
            first: format!("{id} claimed by {}", holders.join(", ")),
        });
    }
    Ok(plan)
}

/// Execute the plan. With `apply == false` nothing is written.
pub async fn run(
    graph: &Graph,
    pool: &PgPool,
    plan: &RekeyPlan,
    apply: bool,
) -> Result<RunReport, RekeyError> {
    let mut report = RunReport::from_plan(plan, apply);
    if !apply {
        tracing::info!(
            nodes = plan.totals().nodes_seen,
            to_rekey = plan.totals().to_rekey,
            refused = plan.totals().refused_shared_key,
            "dry run — planning only, nothing will be written"
        );
        return Ok(report);
    }

    for (doc_slug, nodes) in &plan.by_document {
        let proof = apply_document(graph, pool, doc_slug, nodes).await?;
        match &proof.aborted {
            Some(reason) => tracing::error!(
                document = %doc_slug,
                reason = %reason,
                "document ABORTED and rolled back — its ids are unchanged"
            ),
            None => tracing::info!(
                document = %doc_slug,
                nodes_rekeyed = proof.nodes_rekeyed,
                rows_updated = proof.rows_updated(),
                "document re-keyed"
            ),
        }
        report.documents.push(proof);
    }
    Ok(report)
}

/// Count what each column holds for these old ids, before anything is written.
async fn count_expected(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    old_ids: &[String],
) -> Result<HashMap<String, u64>, RekeyError> {
    const OP: &str = "count_expected";
    let mut expected = HashMap::new();
    for (table, column) in REFERENCING_COLUMNS {
        // The table and column come from a compiled-in constant list, never from
        // input, so the format! cannot carry anything a caller chose. The VALUES
        // are bound.
        let sql = format!("SELECT count(*) FROM {table} WHERE {column} = ANY($1)");
        let count: i64 = sqlx::query_scalar(&sql)
            .bind(old_ids)
            .fetch_one(&mut **tx)
            .await
            .map_err(|source| RekeyError::Postgres {
                operation: OP,
                source,
            })?;
        expected.insert(format!("{table}.{column}"), count as u64);
    }
    Ok(expected)
}

/// Move every old id to its new one, returning what each column actually changed.
async fn apply_updates(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    moves: &[(&str, &str)],
) -> Result<HashMap<String, u64>, RekeyError> {
    const OP: &str = "apply_updates";
    let mut updated = HashMap::new();
    for (table, column) in REFERENCING_COLUMNS {
        let sql = format!("UPDATE {table} SET {column} = $1 WHERE {column} = $2");
        let mut rows = 0u64;
        for (old, new) in moves {
            let result = sqlx::query(&sql)
                .bind(*new)
                .bind(*old)
                .execute(&mut **tx)
                .await
                .map_err(|source| RekeyError::Postgres {
                    operation: OP,
                    source,
                })?;
            rows += result.rows_affected();
        }
        updated.insert(format!("{table}.{column}"), rows);
    }
    Ok(updated)
}

/// One document's unit of work: count, write, verify, then commit or roll back.
async fn apply_document(
    graph: &Graph,
    pool: &PgPool,
    doc_slug: &str,
    nodes: &[PlannedNode],
) -> Result<DocumentProof, RekeyError> {
    const OP: &str = "apply_document";

    let mut proof = DocumentProof {
        doc_slug: doc_slug.to_string(),
        nodes_rekeyed: 0,
        nodes_already_current: nodes
            .iter()
            .filter(|n| matches!(n.disposition, Disposition::AlreadyCurrent))
            .count() as u64,
        nodes_refused: nodes
            .iter()
            .filter(|n| matches!(n.disposition, Disposition::RefusedSharedKey { .. }))
            .count() as u64,
        tables: Vec::new(),
        aborted: None,
    };

    let moves: Vec<(&str, &str)> = nodes
        .iter()
        .filter_map(|n| n.rekey_target().map(|new| (n.row.current_id.as_str(), new)))
        .collect();
    if moves.is_empty() {
        return Ok(proof);
    }
    let old_ids: Vec<String> = moves.iter().map(|(old, _)| (*old).to_string()).collect();

    let mut tx = pool.begin().await.map_err(|source| RekeyError::Postgres {
        operation: OP,
        source,
    })?;

    let expected = count_expected(&mut tx, &old_ids).await?;
    let updated = apply_updates(&mut tx, &moves).await?;

    for (table, column) in REFERENCING_COLUMNS {
        let reference = format!("{table}.{column}");
        proof.tables.push(TableProof {
            expected: expected.get(&reference).copied().unwrap_or(0),
            updated: updated.get(&reference).copied().unwrap_or(0),
            reference,
        });
    }

    // Verify BEFORE committing. A mismatch rolls the whole document back and
    // leaves its ids exactly as they were.
    if let Some(bad) = proof.tables.iter().find(|t| !t.is_sound()) {
        let reason = format!(
            "{}: expected {}, updated {}",
            bad.reference, bad.expected, bad.updated
        );
        tx.rollback().await.map_err(|source| RekeyError::Postgres {
            operation: OP,
            source,
        })?;
        proof.aborted = Some(reason);
        return Ok(proof);
    }

    // The graph side goes LAST: Postgres is verified and ready to commit, so the
    // window in which the two stores disagree is as short as this design allows.
    // The runbook's pre-`--apply` backups are the net under that window, and that
    // is the honest extent of the guarantee (see the module header).
    for (old, new) in &moves {
        graph
            .run(
                query("MATCH (e) WHERE e.id = $old AND labels(e)[0] = $label SET e.id = $new")
                    .param("old", *old)
                    .param("new", *new)
                    .param("label", ENTITY_EVIDENCE),
            )
            .await
            .map_err(|source| RekeyError::Neo4jQuery {
                operation: "rekey_graph_node",
                source,
            })?;
        proof.nodes_rekeyed += 1;
    }

    tx.commit().await.map_err(|source| RekeyError::Postgres {
        operation: OP,
        source,
    })?;

    Ok(proof)
}

#[cfg(test)]
#[path = "execute_tests.rs"]
mod tests;
