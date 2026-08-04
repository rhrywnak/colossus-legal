//! A human's links from a statement to the accusations it bears on (task 2.10).
//!
//! Two tables, one module: `evidence_allegation_links` holds the current state
//! and `evidence_allegation_link_events` holds the append-only record of every
//! link, re-cut and unlink. They are written together, in one transaction, for a
//! reason the ledger's own header states — an unlink leaves NO row in the state
//! table, so without the event a link made and withdrawn is indistinguishable
//! from one never made.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Both tables live in the **pipeline** database (`colossus_legal_v2`), so every
//! call site passes `&state.pipeline_pool`, never `state.pg_pool`.
//!
//! ## Domain note: case-wide, and the missing column says so
//!
//! There is no `scenario_id` anywhere below. A statement that bears on ¶41 bears
//! on ¶41 in every scenario, exactly as the machine's own graph edges do — the
//! same ruling that made `evidence_summary_overrides` case-wide, for the same
//! reason.
//!
//! ## v2 §8: human-authored content
//!
//! This module is the ONLY writer of both tables, and no scan, gather or merge
//! path calls it. That is asserted rather than assumed — both tables are listed
//! in `HUMAN_AUTHORED_TABLES`, and the two scan-path invariant tests in
//! `scenario_human_facts_tests` fail the build if a scan path so much as names
//! one.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::link_cut::{LinkAction, LinkCut};

use super::PipelineRepoError;

/// A row from `evidence_allegation_links`.
///
/// ## Rust learning: `sqlx::FromRow` and why the types are not negotiable
///
/// The derive maps columns to fields BY NAME, and each field's type must be one
/// sqlx can decode from that column's SQL type. `TEXT → String` and
/// `TIMESTAMPTZ → DateTime<Utc>` are the only two pairings here, both already in
/// house use. Nothing in either table is `NUMERIC`: beta.364 died at boot
/// decoding one into an `Option<f64>` with no `rust_decimal` in the tree, and the
/// way that stays fixed is by not introducing the type.
///
/// `cut` is a `String` here rather than a `LinkCut` because this struct is the
/// row as the DATABASE holds it. The token becomes a typed value at the read
/// boundary in the service, where an unknown one can be refused loudly with the
/// ids that carry it — a `FromRow` that parsed it would fail with sqlx's decode
/// error instead, which names the column and not the statement.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EvidenceAllegationLinkRecord {
    pub graph_node_id: String,
    pub allegation_id: String,
    pub cut: String,
    pub authored_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Shared SELECT projection, so the `FromRow` column set cannot drift between
/// query sites (the house anti-drift discipline).
// CONST: column projection locked to the `evidence_allegation_links` schema — a
// structural schema-coupling invariant, not a deployment value.
const LINK_COLUMNS: &str = "graph_node_id, allegation_id, cut, authored_by, created_at, updated_at";

/// Everything needed to write one link.
///
/// A params struct rather than five positional arguments: `graph_node_id`,
/// `allegation_id` and `authored_by` are all `&str`, so a call site could swap
/// two of them with no compile error and file the link against the wrong node.
#[derive(Debug)]
pub struct LinkWrite<'a> {
    pub graph_node_id: &'a str,
    pub allegation_id: &'a str,
    pub cut: LinkCut,
    pub authored_by: &'a str,
    /// Passed in rather than read from the clock here, so every link saved in one
    /// request shares a timestamp and the row matches its log line.
    pub written_at: DateTime<Utc>,
}

// CONST: the upsert. Held as a `const` so a SQL-shape test can pin it without a
// live database (house pattern).
//
// `created_at` is written on insert and deliberately NOT touched on conflict, so
// a link made in June and re-cut today keeps its original date. `updated_at`
// moves.
//
// The RETURNING clause is what makes this one statement rather than two: `xmax`
// is a system column that reads 0 on a freshly inserted row and non-zero on one
// this statement UPDATED, so the caller learns whether it just linked or re-cut
// WITHOUT a preceding SELECT that another request could interleave with. That
// distinction is not cosmetic — it is the difference between a `link` and a
// `recut` row in the ledger.
const UPSERT_LINK_SQL: &str = r#"INSERT INTO evidence_allegation_links
        (graph_node_id, allegation_id, cut, authored_by, created_at, updated_at)
    VALUES ($1, $2, $3, $4, $5, $5)
    ON CONFLICT (graph_node_id, allegation_id) DO UPDATE SET
        cut         = EXCLUDED.cut,
        authored_by = EXCLUDED.authored_by,
        updated_at  = EXCLUDED.updated_at
    RETURNING (xmax <> 0) AS existed"#;

// CONST: the ledger append. Every link, re-cut and unlink lands here.
const INSERT_LINK_EVENT_SQL: &str = r#"INSERT INTO evidence_allegation_link_events
        (graph_node_id, allegation_id, action, cut, actor, at)
    VALUES ($1, $2, $3, $4, $5, $6)"#;

/// Write one link and its ledger entry, in ONE transaction.
///
/// ## Why the two writes are atomic
///
/// The state row says what is true now; the event says a human made it true. A
/// commit that stored one without the other would leave either a link nobody is
/// recorded as having made, or a record of a decision that did not take effect.
/// The rulings ledger holds the same discipline for the same reason.
///
/// Returns the [`LinkAction`] actually performed — `Link` for a pair that had no
/// row, `Recut` for one that did. The caller logs it, and it is what the ledger
/// records.
///
/// ## Rust Learning: `&mut *tx` — passing a transaction as an executor
///
/// `tx` is a `Transaction`, and the query helpers want something implementing
/// `PgExecutor`. `&mut *tx` reborrows through the transaction's `DerefMut` to the
/// connection inside it, which does implement it. Writing `tx` alone would MOVE
/// the transaction into the first query and leave nothing to commit.
///
/// # Errors
/// Returns [`PipelineRepoError`] if either write or the commit fails. Nothing is
/// stored when it does.
pub async fn save_link(
    pool: &PgPool,
    write: &LinkWrite<'_>,
) -> Result<LinkAction, PipelineRepoError> {
    let mut tx = pool.begin().await?;

    let existed: bool = sqlx::query_scalar(UPSERT_LINK_SQL)
        .bind(write.graph_node_id)
        .bind(write.allegation_id)
        .bind(write.cut.code())
        .bind(write.authored_by)
        .bind(write.written_at)
        .fetch_one(&mut *tx)
        .await?;

    let action = if existed {
        LinkAction::Recut
    } else {
        LinkAction::Link
    };

    append_event(&mut tx, write, action).await?;

    tx.commit().await?;
    Ok(action)
}

/// Append one ledger row for a link that was just written.
///
/// Split out so [`save_link`] reads as the sequence of decisions it is, and so
/// the `carries_cut` rule is applied in exactly one place.
async fn append_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    write: &LinkWrite<'_>,
    action: LinkAction,
) -> Result<(), PipelineRepoError> {
    sqlx::query(INSERT_LINK_EVENT_SQL)
        .bind(write.graph_node_id)
        .bind(write.allegation_id)
        .bind(action.code())
        // `carries_cut` is the rule the nullable column encodes, asked rather
        // than assumed — a link and a re-cut both have a cut in force afterwards,
        // and this is the one call site that would have to remember it.
        .bind(action.carries_cut().then(|| write.cut.code()))
        .bind(write.authored_by)
        .bind(write.written_at)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Remove one link and record the withdrawal, in ONE transaction.
///
/// Returns whether a row was actually deleted, so the caller can tell "unlinked"
/// from "there was nothing to unlink" and say so. Collapsing the two would let an
/// unlink on an unlinked pair report work it did not do.
///
/// ## Why the ledger row is written only when something was removed
///
/// A withdrawal that withdrew nothing is not a human decision about this case —
/// it is a stale button in a browser. Recording it would put an event in the
/// record for an act that had no effect, and the ledger's value is that every row
/// in it changed something.
///
/// # Errors
/// Returns [`PipelineRepoError`] if either statement or the commit fails.
pub async fn delete_link(
    pool: &PgPool,
    graph_node_id: &str,
    allegation_id: &str,
    actor: &str,
    at: DateTime<Utc>,
) -> Result<bool, PipelineRepoError> {
    let mut tx = pool.begin().await?;

    let result = sqlx::query(
        "DELETE FROM evidence_allegation_links \
         WHERE graph_node_id = $1 AND allegation_id = $2",
    )
    .bind(graph_node_id)
    .bind(allegation_id)
    .execute(&mut *tx)
    .await?;

    let removed = result.rows_affected() > 0;
    if removed {
        sqlx::query(INSERT_LINK_EVENT_SQL)
            .bind(graph_node_id)
            .bind(allegation_id)
            .bind(LinkAction::Unlink.code())
            // NULL, and this is the only place it is: there is no cut in force
            // after a withdrawal, and writing the old one would make the ledger
            // read as though the unlink had asserted something.
            .bind(Option::<&str>::None)
            .bind(actor)
            .bind(at)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(removed)
}

/// Every human link for a set of statements, in ONE query.
///
/// ## Why a batch read and not one lookup per card
///
/// The cards endpoint composes 148 cards per request. A per-card query would be
/// 148 round trips to answer a question one `= ANY($1)` answers — the same shape
/// every sibling read on this path already uses (`list_fact_refs_for_scenario`,
/// `list_summary_overrides`).
///
/// Ordered by `(graph_node_id, created_at)` so a statement's accusations read in
/// the order the human added them, which is stable across requests. An unordered
/// query would let the card's chips and its composed sentence shuffle between two
/// reads of the same unchanged data.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn list_links_for_nodes(
    pool: &PgPool,
    graph_node_ids: &[String],
) -> Result<Vec<EvidenceAllegationLinkRecord>, PipelineRepoError> {
    // An empty pool is a real state (a scenario nobody has scanned), and
    // `= ANY('{}')` is a round trip that can only return nothing.
    if graph_node_ids.is_empty() {
        return Ok(Vec::new());
    }
    let sql = format!(
        "SELECT {LINK_COLUMNS} FROM evidence_allegation_links \
         WHERE graph_node_id = ANY($1) \
         ORDER BY graph_node_id, created_at, allegation_id"
    );
    let rows = sqlx::query_as::<_, EvidenceAllegationLinkRecord>(&sql)
        .bind(graph_node_ids)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

#[cfg(test)]
#[path = "evidence_allegation_links_tests.rs"]
mod tests;
