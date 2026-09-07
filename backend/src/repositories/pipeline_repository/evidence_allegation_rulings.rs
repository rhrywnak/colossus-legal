//! A human's keep/remove verdict on one machine-ranked item (PROOF_MATRIX_v2 §2).
//!
//! Two tables, one module: `evidence_allegation_rulings` holds the current
//! verdict and `evidence_allegation_ruling_events` holds the append-only record
//! of every ruling, reversal and withdrawal. They are written together, in one
//! transaction, for the reason the ledger's own header states — a withdrawal
//! leaves NO row in the state table, so without the event an item kept and then
//! un-kept is indistinguishable from one nobody ever read.
//!
//! Deliberately the same shape as its sibling
//! [`super::evidence_allegation_links`], down to the `xmax` trick and the
//! params struct. They record different human acts on the same two graph ids,
//! and two modules that look alike are two modules a reader only has to
//! understand once.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Both tables live in the **pipeline** database (`colossus_legal_v2`), so every
//! call site passes `&state.pipeline_pool`, never `state.pg_pool`.
//!
//! ## Domain note: case-wide, and the missing columns say so
//!
//! There is no `scenario_id` and no `count_number`. An item either belongs under
//! ¶41 or it does not — and the same allegation is reached from several Counts
//! through several Elements, so a per-Count key would let one decision mean two
//! different things depending on which Element the reader happened to open.
//!
//! ## v2 §8: human-authored content
//!
//! This module is the ONLY writer of both tables, and no scan, gather or merge
//! path calls it. That is asserted rather than assumed — both tables are listed
//! in `HUMAN_AUTHORED_TABLES`, and the two scan-path invariant tests in
//! `scenario_human_facts_tests` fail the build if a scan path so much as names
//! one.
//!
//! ## Two callers, both human clicks (integration ruling R1, 2026-09-07)
//!
//! [`save_ruling`] is called from `api::proof_matrix_rulings` — a human ruling on
//! the Matrix page — and from `api::scenario_fact_include`, where a human
//! INCLUDING a fact under an accusation has made the same judgment on another
//! page and should not be asked for it twice.
//!
//! Both are a person clicking, which is what the §8 rule is about; neither is a
//! machine path. The second caller writes `Keep` and never `Remove`, and it
//! commits in its OWN transaction so a failure here cannot roll back the link the
//! scenario page reads.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::matrix_ruling::{MatrixRuling, RulingAction};

use super::PipelineRepoError;

/// A row from `evidence_allegation_rulings`.
///
/// ## Rust learning: `sqlx::FromRow` and why the types are not negotiable
///
/// The derive maps columns to fields BY NAME, and each field's type must be one
/// sqlx can decode from that column's SQL type. `TEXT → String` and
/// `TIMESTAMPTZ → DateTime<Utc>` are the only pairings here. Nothing in either
/// table is `NUMERIC`: beta.364 died at boot decoding one into an `Option<f64>`
/// with no `rust_decimal` in the tree, and the way that stays fixed is by not
/// introducing the type.
///
/// `ruling` is a `String` here rather than a [`MatrixRuling`] because this struct
/// is the row as the DATABASE holds it. The token becomes a typed value at the
/// read boundary in [`RulingRecord::verdict`], where an unknown one can be
/// refused loudly with the ids that carry it — a `FromRow` that parsed it would
/// fail with sqlx's decode error instead, which names the column and not the item.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RulingRecord {
    pub evidence_id: String,
    pub allegation_id: String,
    pub ruling: String,
    pub ruled_by: String,
    pub ruled_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub note: Option<String>,
}

impl RulingRecord {
    /// The stored token as a typed verdict, or a named error.
    ///
    /// ## Why this is a method and not a `TryFrom` on the whole record
    ///
    /// The caller has the ids in hand and can say WHICH item carries the bad
    /// token. A conversion that consumed the record would have to rebuild that
    /// context, and a proof surface that says "one of these is unreadable"
    /// without saying which is not much better than saying nothing.
    ///
    /// # Errors
    /// Returns [`PipelineRepoError::Database`] naming the pair and the token when
    /// the column holds a word this build does not define.
    pub fn verdict(&self) -> Result<MatrixRuling, PipelineRepoError> {
        MatrixRuling::try_from(self.ruling.as_str()).map_err(|e| {
            PipelineRepoError::Database(format!(
                "evidence_allegation_rulings holds an unreadable verdict for \
                 (evidence {}, allegation {}): {e}",
                self.evidence_id, self.allegation_id
            ))
        })
    }
}

/// Shared SELECT projection, so the `FromRow` column set cannot drift between
/// query sites (the house anti-drift discipline).
// CONST: column projection locked to the `evidence_allegation_rulings` schema — a
// structural schema-coupling invariant, not a deployment value.
const RULING_COLUMNS: &str =
    "evidence_id, allegation_id, ruling, ruled_by, ruled_at, updated_at, note";

/// Everything needed to write one ruling.
///
/// A params struct rather than five positional arguments: `evidence_id`,
/// `allegation_id` and `ruled_by` are all `&str`, so a call site could swap two
/// of them with no compile error and file the verdict against the wrong item.
#[derive(Debug)]
pub struct RulingWrite<'a> {
    pub evidence_id: &'a str,
    pub allegation_id: &'a str,
    pub ruling: MatrixRuling,
    pub ruled_by: &'a str,
    /// An optional sentence from the human. `None` is the ordinary state.
    pub note: Option<&'a str>,
    /// Passed in rather than read from the clock here, so every ruling saved in
    /// one request shares a timestamp and the row matches its log line.
    pub written_at: DateTime<Utc>,
}

// CONST: the upsert. Held as a `const` so a SQL-shape test can pin it without a
// live database (house pattern).
//
// `ruled_at` is written on insert and deliberately NOT touched on conflict, so an
// item kept in June and reversed today keeps its original date. `updated_at`
// moves.
//
// The RETURNING clause is what makes this one statement rather than two: `xmax`
// is a system column that reads 0 on a freshly inserted row and non-zero on one
// this statement UPDATED, so the caller learns whether it just ruled or reversed
// WITHOUT a preceding SELECT that another request could interleave with. That
// distinction is not cosmetic — it is the difference between a `rule` and a
// `rerule` row in the ledger.
const UPSERT_RULING_SQL: &str = r#"INSERT INTO evidence_allegation_rulings
        (evidence_id, allegation_id, ruling, ruled_by, ruled_at, updated_at, note)
    VALUES ($1, $2, $3, $4, $5, $5, $6)
    ON CONFLICT (evidence_id, allegation_id) DO UPDATE SET
        ruling     = EXCLUDED.ruling,
        ruled_by   = EXCLUDED.ruled_by,
        updated_at = EXCLUDED.updated_at,
        note       = EXCLUDED.note
    RETURNING (xmax <> 0) AS existed"#;

// CONST: the ledger append. Every ruling, reversal and withdrawal lands here.
const INSERT_RULING_EVENT_SQL: &str = r#"INSERT INTO evidence_allegation_ruling_events
        (evidence_id, allegation_id, action, ruling, note, actor, at)
    VALUES ($1, $2, $3, $4, $5, $6, $7)"#;

/// Write one ruling and its ledger entry, in ONE transaction.
///
/// ## Why the two writes are atomic
///
/// The state row says what is true now; the event says a human made it true. A
/// commit that stored one without the other would leave either a verdict nobody
/// is recorded as having reached, or a record of a decision that did not take
/// effect. On a surface whose whole claim is "a human has been through this",
/// either half alone is a lie.
///
/// Returns the [`RulingAction`] actually performed — `Rule` for a pair that had
/// no row, `Rerule` for one that did. The caller logs it, and it is what the
/// ledger records.
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
pub async fn save_ruling(
    pool: &PgPool,
    write: &RulingWrite<'_>,
) -> Result<RulingAction, PipelineRepoError> {
    let mut tx = pool.begin().await?;

    let existed: bool = sqlx::query_scalar(UPSERT_RULING_SQL)
        .bind(write.evidence_id)
        .bind(write.allegation_id)
        .bind(write.ruling.code())
        .bind(write.ruled_by)
        .bind(write.written_at)
        .bind(write.note)
        .fetch_one(&mut *tx)
        .await?;

    let action = if existed {
        RulingAction::Rerule
    } else {
        RulingAction::Rule
    };

    append_event(&mut tx, write, action).await?;

    tx.commit().await?;
    Ok(action)
}

/// Append one ledger row for a ruling that was just written.
///
/// Split out so [`save_ruling`] reads as the sequence of decisions it is, and so
/// the `carries_ruling` rule is applied in exactly one place.
async fn append_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    write: &RulingWrite<'_>,
    action: RulingAction,
) -> Result<(), PipelineRepoError> {
    sqlx::query(INSERT_RULING_EVENT_SQL)
        .bind(write.evidence_id)
        .bind(write.allegation_id)
        .bind(action.code())
        // `carries_ruling` is the rule the nullable column encodes, asked rather
        // than assumed — a ruling and a reversal both leave a verdict in force,
        // and this is the one call site that would have to remember it.
        .bind(action.carries_ruling().then(|| write.ruling.code()))
        .bind(write.note)
        .bind(write.ruled_by)
        .bind(write.written_at)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Withdraw one ruling and record the withdrawal, in ONE transaction.
///
/// Returns whether a row was actually deleted, so the caller can tell "withdrawn"
/// from "there was nothing to withdraw" and say so. Collapsing the two would let
/// an Undo on an unruled item report work it did not do.
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
pub async fn withdraw_ruling(
    pool: &PgPool,
    evidence_id: &str,
    allegation_id: &str,
    actor: &str,
    at: DateTime<Utc>,
) -> Result<bool, PipelineRepoError> {
    let mut tx = pool.begin().await?;

    let result = sqlx::query(
        "DELETE FROM evidence_allegation_rulings \
         WHERE evidence_id = $1 AND allegation_id = $2",
    )
    .bind(evidence_id)
    .bind(allegation_id)
    .execute(&mut *tx)
    .await?;

    let removed = result.rows_affected() > 0;
    if removed {
        sqlx::query(INSERT_RULING_EVENT_SQL)
            .bind(evidence_id)
            .bind(allegation_id)
            .bind(RulingAction::Withdraw.code())
            // NULL, and this is the only place it is: there is no verdict in
            // force after a withdrawal, and writing the old one would make the
            // ledger read as though the withdrawal had asserted something.
            .bind(Option::<&str>::None)
            .bind(Option::<&str>::None)
            .bind(actor)
            .bind(at)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(removed)
}

/// Every ruling bearing on a set of accusations, in ONE query.
///
/// ## Why keyed by allegation and not by evidence
///
/// The Matrix asks "what has a human said about the items under ¶41" — it holds
/// the allegation and discovers the items. Reading by evidence id would mean
/// collecting every id on the page first and then a second pass to attach them,
/// which is the same round trip with an extra step.
///
/// Ordered by `(allegation_id, evidence_id)` so two reads of unchanged data
/// return the same order. An unordered query would let the drill-down's rows
/// shuffle between refreshes, on a page whose ORDER is a claim.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn list_rulings_for_allegations(
    pool: &PgPool,
    allegation_ids: &[String],
) -> Result<Vec<RulingRecord>, PipelineRepoError> {
    // An empty list is a real state (an Element nothing bears on), and
    // `= ANY('{}')` is a round trip that can only return nothing.
    if allegation_ids.is_empty() {
        return Ok(Vec::new());
    }
    let sql = format!(
        "SELECT {RULING_COLUMNS} FROM evidence_allegation_rulings \
         WHERE allegation_id = ANY($1) \
         ORDER BY allegation_id, evidence_id"
    );
    let rows = sqlx::query_as::<_, RulingRecord>(&sql)
        .bind(allegation_ids)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

#[cfg(test)]
#[path = "evidence_allegation_rulings_tests.rs"]
mod tests;
