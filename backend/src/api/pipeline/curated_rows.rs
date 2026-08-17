//! `GET /documents/:id/curated-rows` — how much human ruling a document carries.
//!
//! ## Why this endpoint exists (the honesty law)
//!
//! Re-extraction can move Evidence ids. With the stable-id arm live it usually
//! does not, but "usually" is not a guarantee an operator can act on: if the
//! model returns a quote one word longer, that statement's id moves and every
//! curated row pointing at it dangles. The dialog therefore has to be able to
//! say, before the operator commits, how much is at stake — measured, not
//! adjectival.
//!
//! Measured 2026-08-17, and it is why the friction is conditional: the two
//! affidavits and the Tighe opinion carry **zero** curated rows, while the Court
//! of Appeals ruling carries **225** across seven columns. The same warning on
//! both would be noise on one and inadequate on the other.
//!
//! ## What it counts
//!
//! The ten CURATED columns of the eleven-column registry
//! ([`EVIDENCE_CURATED_REFERENCES`]) — not `extraction_items.neo4j_node_id`,
//! which is pipeline provenance the re-extraction rewrites itself. Counting that
//! would make every document look like it carried rulings.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::models::graph_refs::EVIDENCE_CURATED_REFERENCES;
use crate::state::AppState;

/// One column's contribution, so the dialog can name what is at stake.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CuratedColumnCount {
    /// `table.column`, the form every proof line in this project uses.
    pub reference: String,
    pub rows: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CuratedRowsResponse {
    pub document_id: String,
    /// Sum across every curated column. `0` means no friction is needed.
    pub total: i64,
    /// Only the columns that actually carry rows, most first — a dialog listing
    /// seven zeroes teaches an operator to stop reading it.
    pub by_column: Vec<CuratedColumnCount>,
}

/// Order the columns biggest-first, ties broken alphabetically.
///
/// A free function rather than an inline `sort_by` because the ORDER is
/// load-bearing: the dialog shows this list on one line, and an operator
/// skimming it should meet the number that matters rather than the
/// alphabetically luckiest table. The tie-break is there so two columns with
/// equal counts do not swap places between requests and make the dialog look
/// unstable.
pub(crate) fn sort_biggest_first(by_column: &mut [CuratedColumnCount]) {
    by_column.sort_by(|a, b| b.rows.cmp(&a.rows).then(a.reference.cmp(&b.reference)));
}

/// Count the curated rows anchored to this document's Evidence nodes.
///
/// ## Rust Learning: why the table name is interpolated and the id is bound
///
/// A table or column name cannot be a bind parameter in SQL — only a value can.
/// The names here come from [`EVIDENCE_CURATED_REFERENCES`], a `const` in this
/// binary, so nothing an operator types can reach the interpolation; the
/// document id, which IS operator input, goes through `$1` where it can only
/// ever be a value. That split is the whole injection story on this path.
pub async fn curated_rows_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<CuratedRowsResponse>, AppError> {
    require_admin(&user)?;

    let mut by_column: Vec<CuratedColumnCount> = Vec::new();
    let mut total: i64 = 0;

    for c in EVIDENCE_CURATED_REFERENCES {
        // The document's Evidence node ids come from `extraction_items`, which is
        // the same join `remap_evidence` uses. Reading them per column rather
        // than once keeps the statement a single round trip and lets Postgres
        // plan each count independently.
        let sql = format!(
            "SELECT count(*) FROM {} r \
             WHERE r.{} IN ( \
                 SELECT neo4j_node_id FROM extraction_items \
                 WHERE document_id = $1 AND neo4j_node_id IS NOT NULL \
             )",
            c.table, c.column
        );

        let rows: i64 = sqlx::query_scalar(&sql)
            .bind(&doc_id)
            .fetch_one(&state.pipeline_pool)
            .await
            .map_err(|e| AppError::Internal {
                message: format!(
                    "Failed to count curated rows in {} for '{doc_id}': {e}. The \
                     re-extraction guard cannot state what is at stake, so the dialog \
                     will refuse rather than under-report. Check PIPELINE_DATABASE_URL.",
                    c.reference()
                ),
            })?;

        if rows > 0 {
            total += rows;
            by_column.push(CuratedColumnCount {
                reference: c.reference(),
                rows,
            });
        }
    }

    sort_biggest_first(&mut by_column);

    tracing::info!(
        doc_id = %doc_id,
        total,
        columns = by_column.len(),
        "Curated-row count served for the re-extraction guard"
    );

    Ok(Json(CuratedRowsResponse {
        document_id: doc_id,
        total,
        by_column,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(reference: &str, rows: i64) -> CuratedColumnCount {
        CuratedColumnCount {
            reference: reference.to_string(),
            rows,
        }
    }

    /// Biggest first, and a tie broken alphabetically so the dialog does not
    /// reshuffle between two requests over identical data.
    #[test]
    fn columns_are_ordered_biggest_first_with_a_stable_tie_break() {
        // The live Court of Appeals shape, plus a deliberate tie on 46.
        let mut v = vec![
            c("scenario_fact_refs.graph_node_id", 9),
            c("scan_run_verdicts.graph_node_id", 60),
            c("zz_last_alphabetically.graph_node_id", 46),
            c("scenario_ruling_anchors.graph_node_id", 46),
            c("scenario_candidate_ordinals.graph_node_id", 99),
        ];
        sort_biggest_first(&mut v);

        let order: Vec<&str> = v.iter().map(|x| x.reference.as_str()).collect();
        assert_eq!(
            order,
            vec![
                "scenario_candidate_ordinals.graph_node_id", // 99
                "scan_run_verdicts.graph_node_id",           // 60
                "scenario_ruling_anchors.graph_node_id",     // 46, alphabetically first
                "zz_last_alphabetically.graph_node_id",      // 46
                "scenario_fact_refs.graph_node_id",          // 9
            ],
        );
    }

    #[test]
    fn an_empty_list_sorts_without_panicking() {
        let mut v: Vec<CuratedColumnCount> = Vec::new();
        sort_biggest_first(&mut v);
        assert!(v.is_empty());
    }
}
