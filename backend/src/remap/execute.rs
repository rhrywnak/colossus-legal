//! Reading the two stores for the remap, and applying an approved proposal.
//!
//! ## Why the apply step touches only Postgres
//!
//! By the time a remap runs, the reprocess has already happened: the old Evidence
//! nodes are gone and the new ones exist. Nothing about the graph needs changing.
//! What is left behind is curated Postgres rows pointing at ids that no longer
//! resolve, and moving those is the whole job — which is why the unit of work is
//! one document in one transaction, and why a mismatch rolls the whole document
//! back rather than leaving half a document's rulings moved.

use std::fmt::Write as _;

use neo4rs::{query, Graph};
use sqlx::PgPool;

use super::plan::{NewNode, Snapshot, SnapshotNode};
use crate::models::document_status::ENTITY_EVIDENCE;
use crate::neo4j::schema;
use crate::oneshot::refs::{
    count_rows, repoint, table_proofs, EVIDENCE_CURATED_REFERENCES, EVIDENCE_REFERENCES,
};

/// Why the remap could not proceed.
#[derive(Debug, thiserror::Error)]
pub enum RemapError {
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
}

/// Read every Evidence node in one document, with its key fields.
pub async fn load_document_nodes(
    graph: &Graph,
    document_id: &str,
) -> Result<Vec<NewNode>, RemapError> {
    const OP: &str = "load_document_nodes";
    // Interpolated from `neo4j::schema`; a relationship type cannot be a bind
    // parameter, and a bare literal here would survive any rename.
    let cypher = format!(
        "MATCH (e)-[:{contained_in}]->(d) \
         WHERE labels(e)[0] = $label AND d.id = $document \
         RETURN e.id AS id, e.page_number AS page, e.verbatim_quote AS quote, \
                e.question AS question \
         ORDER BY e.id",
        contained_in = schema::CONTAINED_IN
    );

    let mut stream = graph
        .execute(
            query(&cypher)
                .param("label", ENTITY_EVIDENCE)
                .param("document", document_id),
        )
        .await
        .map_err(|source| RemapError::Neo4jQuery {
            operation: OP,
            source,
        })?;

    let mut nodes = Vec::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(|source| RemapError::Neo4jQuery {
            operation: OP,
            source,
        })?
    {
        let decode = |e: neo4rs::DeError| RemapError::Neo4jDecode {
            operation: OP,
            source: e,
        };
        nodes.push(NewNode {
            id: row.get("id").map_err(decode)?,
            page: row.get("page").map_err(decode)?,
            verbatim_quote: row
                .get::<Option<String>>("quote")
                .map_err(decode)?
                .unwrap_or_default(),
            question: row.get("question").map_err(decode)?,
        });
    }
    Ok(nodes)
}

/// Capture one document's state BEFORE a reprocess.
///
/// `taken_note` is free text carried into the file so a reader of a stale
/// snapshot can tell it is stale. It is never parsed.
pub async fn take_snapshot(
    graph: &Graph,
    pool: &PgPool,
    document_id: &str,
    taken_note: &str,
) -> Result<Snapshot, RemapError> {
    let nodes = load_document_nodes(graph, document_id).await?;
    let ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let curated = count_curated_per_node(pool, &ids).await?;

    Ok(Snapshot {
        document_id: document_id.to_string(),
        taken_note: taken_note.to_string(),
        nodes: nodes
            .into_iter()
            .map(|n| SnapshotNode {
                curated_rows: curated
                    .iter()
                    .find(|(id, _)| id == &n.id)
                    .map(|(_, c)| *c)
                    .unwrap_or(0),
                id: n.id,
                page: n.page,
                verbatim_quote: n.verbatim_quote,
                question: n.question,
            })
            .collect(),
    })
}

/// Count, per node, how many CURATED rows reference it.
///
/// Only the curated columns: `extraction_items.neo4j_node_id` is pipeline
/// provenance that the reprocess rewrites itself, and counting it here would
/// make every node look like it carried a ruling.
async fn count_curated_per_node(
    pool: &PgPool,
    ids: &[String],
) -> Result<Vec<(String, u64)>, RemapError> {
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
            .map_err(|source| RemapError::Postgres {
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

/// What one referencing column did — the family-wide proof type.
///
/// Re-exported rather than redefined: all four one-shot tools count the same
/// thing the same way, and three copies of one struct is how a `!=` quietly
/// becomes a `>=` in one of them. See [`crate::oneshot::refs::TableProof`].
pub use crate::oneshot::refs::TableProof;

/// What the apply step did.
#[derive(Debug, Clone)]
pub struct ApplyReport {
    pub document_id: String,
    pub approved_by: String,
    pub moves: usize,
    pub tables: Vec<TableProof>,
    pub aborted: Option<String>,
}

impl ApplyReport {
    pub fn rows_updated(&self) -> u64 {
        self.tables.iter().map(|t| t.updated).sum()
    }

    /// Render the count proof.
    pub fn render(&self) -> String {
        let mut out = String::new();
        let state = match &self.aborted {
            Some(reason) => format!("ABORTED — rolled back: {reason}"),
            None => "APPLIED".to_string(),
        };
        let _ = writeln!(out, "=== EVIDENCE REMAP — {state} ===\n");
        let _ = writeln!(out, "  document      : {}", self.document_id);
        let _ = writeln!(out, "  approved by   : {}", self.approved_by);
        let _ = writeln!(out, "  moves applied : {}", self.moves);
        let _ = writeln!(out, "  rows updated  : {}\n", self.rows_updated());
        for table in &self.tables {
            let mismatch = if table.is_sound() {
                ""
            } else {
                "   <-- MISMATCH"
            };
            let _ = writeln!(
                out,
                "  {:<48} expected {:>4}  updated {:>4}{}",
                table.reference, table.expected, table.updated, mismatch
            );
        }
        out
    }
}

/// Apply an approved proposal: one document, one transaction.
///
/// The moves come from [`super::proposal::Proposal::approved_moves`], which
/// refuses an unapproved file — so this function cannot be reached with a
/// proposal nobody read.
pub async fn apply(
    pool: &PgPool,
    document_id: &str,
    approved_by: &str,
    moves: &[(String, String)],
) -> Result<ApplyReport, RemapError> {
    const OP: &str = "apply";
    let old_ids: Vec<String> = moves.iter().map(|(old, _)| old.clone()).collect();

    let mut tx = pool.begin().await.map_err(|source| RemapError::Postgres {
        operation: OP,
        source,
    })?;
    let expected = count_rows(&mut tx, EVIDENCE_REFERENCES, &old_ids)
        .await
        .map_err(|source| RemapError::Postgres {
            operation: "count_rows",
            source,
        })?;
    let updated = repoint(&mut tx, EVIDENCE_REFERENCES, moves)
        .await
        .map_err(|source| RemapError::Postgres {
            operation: "repoint",
            source,
        })?;

    let tables = table_proofs(EVIDENCE_REFERENCES, &expected, &updated);

    let mut report = ApplyReport {
        document_id: document_id.to_string(),
        approved_by: approved_by.to_string(),
        moves: moves.len(),
        tables,
        aborted: None,
    };

    if let Some(bad) = report.tables.iter().find(|t| !t.is_sound()) {
        let reason = format!(
            "{}: expected {}, updated {}",
            bad.reference, bad.expected, bad.updated
        );
        tx.rollback().await.map_err(|source| RemapError::Postgres {
            operation: OP,
            source,
        })?;
        report.aborted = Some(reason);
        return Ok(report);
    }

    tx.commit().await.map_err(|source| RemapError::Postgres {
        operation: OP,
        source,
    })?;
    Ok(report)
}

#[cfg(test)]
#[path = "execute_tests.rs"]
mod tests;
