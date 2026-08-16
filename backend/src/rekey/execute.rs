//! Reading the graph and applying the re-key, one document at a time.
//!
//! Everything that touches a store lives here; every DECISION lives in
//! [`super::plan`]. The split is what makes the plan unit-testable and keeps this
//! file to the mechanics: read, count, write, verify, commit or abort.
//!
//! ## The eleven referencing columns
//!
//! Ruled 2026-08-16, correcting this tool's original list of eight. There are
//! ELEVEN columns holding an Evidence graph id, and the re-key updates all of
//! them. They live in [`crate::oneshot::refs::EVIDENCE_REFERENCES`] — the same
//! single registry the twin merge, the remap and the party merge walk, so a
//! column can never be known to one tool and invisible to another.
//!
//! ## What the correction was, and what it cost
//!
//! The original eight were the columns Phase A measured as POPULATED on
//! 2026-08-14. Three were missed:
//!
//! - `evidence_summary_overrides.graph_node_id` and
//!   `response_item_fact_refs.graph_node_id` — real curated surfaces that
//!   happened to hold zero rows that week. Empty is a fact about a Tuesday, not
//!   a property of the schema.
//! - `extraction_items.neo4j_node_id` — **not empty**. It holds 525 Evidence
//!   ids, and it is READ: `lookup_neo4j_node_ids` resolves cross-document
//!   references from it at ingest, and pass-2 prefers it over re-resolving. A
//!   re-key that skipped it left 483 rows pointing at ids that no longer exist.
//!
//! The cost is a bigger number in the count proof, and only that: **1,318 rows
//! across eleven columns** where the eight-column version moved 835. Both
//! figures are measured, not derived — see the constant's own doc for the
//! per-column breakdown.
//!
//! A column added to the schema and forgotten would leave dangling rows the
//! count proof could not see, because the proof walks this same list. That is
//! why membership is pinned by a test against a dated `information_schema`
//! sweep rather than trusted to review.

use neo4rs::{query, Graph};
use sqlx::PgPool;

use super::plan::{Disposition, EvidenceRow, PlannedNode, RekeyPlan};
use super::report::{DocumentProof, RunReport};
use crate::models::document_status::ENTITY_EVIDENCE;
use crate::oneshot::refs::{count_rows, repoint, table_proofs, EVIDENCE_REFERENCES};

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

    // Owned pairs, because the shared `repoint` takes `&[(String, String)]` —
    // one signature for all four tools. The clone is per re-keyed node, once,
    // which is nothing against a run that opens a transaction per document.
    let moves: Vec<(String, String)> = nodes
        .iter()
        .filter_map(|n| {
            n.rekey_target()
                .map(|new| (n.row.current_id.clone(), new.to_string()))
        })
        .collect();
    if moves.is_empty() {
        return Ok(proof);
    }
    let old_ids: Vec<String> = moves.iter().map(|(old, _)| old.clone()).collect();

    let mut tx = pool.begin().await.map_err(|source| RekeyError::Postgres {
        operation: OP,
        source,
    })?;

    let expected = count_rows(&mut tx, EVIDENCE_REFERENCES, &old_ids)
        .await
        .map_err(|source| RekeyError::Postgres {
            operation: "count_rows",
            source,
        })?;
    let updated = repoint(&mut tx, EVIDENCE_REFERENCES, &moves)
        .await
        .map_err(|source| RekeyError::Postgres {
            operation: "repoint",
            source,
        })?;
    proof.tables = table_proofs(EVIDENCE_REFERENCES, &expected, &updated);

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
                    .param("old", old.as_str())
                    .param("new", new.as_str())
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
