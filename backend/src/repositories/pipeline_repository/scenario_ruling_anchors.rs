//! Repository for `scenario_ruling_anchors` in the `colossus_legal_v2` pipeline
//! database — the append-only ledger of every ruling, with the durable anchor
//! recorded at ruling time.
//!
//! ## Why this is a separate module from `scenario_store`
//!
//! `scenario_store` writes STATE: it upserts the one live row per
//! (scenario, candidate) that says what the current decision is. This module
//! writes HISTORY: one row per ruling event, never updated, never deleted. The
//! two have opposite disciplines — an upsert here would destroy the record, and
//! an append there would break the composite-PK invariant `join_facts` depends on
//! — so keeping them in separate modules keeps the disciplines from bleeding.
//!
//! ## The one rule this module enforces on its own
//!
//! The `*_state` columns are DERIVED here from the shape of their value columns,
//! never accepted from the caller. A caller could otherwise pass
//! `speaker: None, speaker_state: Present` and store an anchor that lies about
//! its own completeness. Deriving them at the single write point makes that
//! unrepresentable rather than merely discouraged.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::ruling_anchor::{normalize_quote, FieldPresence, RulingKind};

use super::PipelineRepoError;

/// One ruling to append to the ledger, with everything known at ruling time.
///
/// ## Rust Learning: a params struct instead of a twelve-argument function
///
/// The sibling writers (`upsert_fact_ref`, `insert_scenario`) carry an
/// `#[allow(clippy::too_many_arguments)]` because their arguments are a short list
/// of distinct column values. This one crosses the line where positional arguments
/// stop being readable: `(…, None, Some(q), None, …)` at a call site tells the
/// reader nothing about which field is which, and two `Option<String>` neighbours
/// can be swapped with no compile error. A struct makes every value named at the
/// call site, so a mis-assignment is visible in review and most swaps become type
/// errors.
///
/// Borrowed `&str` fields rather than owned `String`: the caller already holds
/// these (they came out of the graph read), so borrowing avoids cloning the quote
/// — which is the largest field here — purely to hand it to a function that only
/// binds it.
#[derive(Debug)]
pub struct RulingAnchorWrite<'a> {
    /// The scenario the ruling was made in.
    pub scenario_id: Uuid,
    /// The graph node id AS IT WAS at ruling time. Provenance and the task-2.5
    /// fast path — never the durable key.
    pub graph_node_id: &'a str,
    /// Which ruling this was.
    pub kind: RulingKind,
    /// The source document, when it could be read.
    ///
    /// `Option` only because a REMOVAL may be recorded after the graph node has
    /// vanished, at which point nothing about the item is readable. Every other
    /// ruling is refused without a document before it ever reaches this struct
    /// (`RulingKind::requires_document`), so an `Include` here always carries one.
    pub document_id: Option<&'a str>,
    /// The page, when the record carries one.
    pub page: Option<i64>,
    /// The verbatim words, when the record carries them. Mandatory for include
    /// and exclude — enforced by the write path, not here.
    pub quote_verbatim: Option<&'a str>,
    /// The speaker, when the record carries one.
    pub speaker: Option<&'a str>,
    /// When the ruling was made. Passed IN rather than read from the clock here,
    /// so the caller's log line, the state row and the ledger row all carry the
    /// same instant — and so tests are deterministic.
    pub ruled_at: DateTime<Utc>,
    /// Who made it — the authenticated username, or a machine path's identifier.
    pub ruled_by: &'a str,
    /// Why it was deferred. Required for defer, absent otherwise; the database
    /// CHECK ties the two together in both directions.
    pub defer_reason: Option<&'a str>,
}

// CONST: the append statement. Held as a `const` so a SQL-shape unit test can pin
// its append-only semantics without a live database (house pattern, as in
// `scenario_candidate_ordinals`). Query text, not deployment config — Rule 13 does
// not apply.
//
// Note there is NO `ON CONFLICT` clause, and that is the point: the surrogate
// `anchor_id` means two rulings on the same candidate never collide, so every
// ruling appends. An `ON CONFLICT … DO UPDATE` here would silently convert the
// ledger back into the overwriting state table this exists to complement.
const INSERT_ANCHOR_SQL: &str = r#"INSERT INTO scenario_ruling_anchors
        (scenario_id, graph_node_id, ruled_status, document_id, document_state,
         page, page_state, quote_verbatim, quote_normalized, quote_state,
         speaker, speaker_state, ruled_at, ruled_by, defer_reason)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
    RETURNING anchor_id"#;

/// Append one ruling to the ledger and return its `anchor_id`.
///
/// The `*_state` columns and `quote_normalized` are computed here from the
/// supplied values — see the module doc for why they are not caller-supplied.
///
/// ## Rust Learning: `impl sqlx::PgExecutor<'_>` so the caller can use a transaction
///
/// Taking a generic executor rather than `&PgPool` lets the caller pass either a
/// pool (standalone write) or `&mut *tx` (inside a transaction). The ruling write
/// path uses the latter, so the state row and this ledger row commit as ONE unit:
/// a ruling can never be recorded-without-anchoring or anchored-without-recording.
/// That atomicity is the whole reason the anchor cannot drift from the state.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the insert fails — notably the
/// `defer_reason IFF defer` CHECK, which fires if a caller ever assembles a defer
/// with no reason (or a reason on a non-defer) despite the code-level guard.
pub async fn insert_ruling_anchor(
    executor: impl sqlx::PgExecutor<'_>,
    anchor: &RulingAnchorWrite<'_>,
) -> Result<Uuid, PipelineRepoError> {
    // Derived, never accepted: a state that contradicts its value column would be
    // an anchor lying about its own completeness.
    let document_state = FieldPresence::of(&anchor.document_id);
    let page_state = FieldPresence::of(&anchor.page);
    let quote_state = FieldPresence::of(&anchor.quote_verbatim);
    let speaker_state = FieldPresence::of(&anchor.speaker);

    // Normalize once, here, so every row in the ledger was normalized by exactly
    // one code path under exactly one version of the law. A caller computing its
    // own normalized form could drift from this one silently.
    let quote_normalized = anchor.quote_verbatim.map(normalize_quote);

    let anchor_id = sqlx::query_scalar::<_, Uuid>(INSERT_ANCHOR_SQL)
        .bind(anchor.scenario_id)
        .bind(anchor.graph_node_id)
        .bind(anchor.kind.code())
        .bind(anchor.document_id)
        .bind(document_state.code())
        .bind(anchor.page)
        .bind(page_state.code())
        .bind(anchor.quote_verbatim)
        .bind(quote_normalized)
        .bind(quote_state.code())
        .bind(anchor.speaker)
        .bind(speaker_state.code())
        .bind(anchor.ruled_at)
        .bind(anchor.ruled_by)
        .bind(anchor.defer_reason)
        .fetch_one(executor)
        .await?;
    Ok(anchor_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The statement must APPEND, never upsert.
    ///
    /// This is the single most important property of the ledger: the 2026-07-24
    /// loss happened because the only record of a ruling was overwritable. An
    /// `ON CONFLICT` clause appearing here would mean a second ruling on the same
    /// candidate replaces the first, and the history this table exists to hold
    /// would be gone.
    #[test]
    fn the_insert_is_append_only() {
        assert!(
            !INSERT_ANCHOR_SQL.contains("ON CONFLICT"),
            "the anchor ledger must never upsert — one row per ruling EVENT: {INSERT_ANCHOR_SQL}"
        );
        assert!(
            INSERT_ANCHOR_SQL.trim_start().starts_with("INSERT INTO"),
            "the ledger write must be a plain INSERT: {INSERT_ANCHOR_SQL}"
        );
        assert!(
            !INSERT_ANCHOR_SQL.contains("UPDATE") && !INSERT_ANCHOR_SQL.contains("DELETE"),
            "the ledger is append-only: no UPDATE, no DELETE: {INSERT_ANCHOR_SQL}"
        );
    }

    /// Every column the table requires is bound, and the arity matches.
    ///
    /// A dropped bind would shift every later value one column left — the exact
    /// defect class `sql_invariants` exists to catch, asserted here directly for
    /// the statement that records forensic history.
    #[test]
    fn every_anchor_column_is_bound_once() {
        for column in [
            "scenario_id",
            "graph_node_id",
            "ruled_status",
            "document_id",
            "document_state",
            "page",
            "page_state",
            "quote_verbatim",
            "quote_normalized",
            "quote_state",
            "speaker",
            "speaker_state",
            "ruled_at",
            "ruled_by",
            "defer_reason",
        ] {
            assert!(
                INSERT_ANCHOR_SQL.contains(column),
                "the ledger insert must write {column}"
            );
        }
        // Fifteen columns, fifteen placeholders. The highest is $15, and $16 must
        // not appear — a dropped bind would shift every later value one column
        // left, which is exactly the defect class `sql_invariants` polices.
        assert!(
            INSERT_ANCHOR_SQL.contains("$15"),
            "expected 15 placeholders"
        );
        assert!(
            !INSERT_ANCHOR_SQL.contains("$16"),
            "more placeholders than columns"
        );
    }

    /// The document column is written with its own state, like page and speaker.
    ///
    /// `document_id` became nullable so a REMOVAL can be recorded against a node
    /// the graph has already lost. That makes the paired state column
    /// load-bearing: it is the difference between "this removal had no readable
    /// document" and "someone forgot to bind the column". The migration backs it
    /// with a CHECK tying the two together, because `document_id` is the field
    /// task 2.5 indexes on — a row claiming `present` over a NULL would be found
    /// by that index and then match nothing.
    #[test]
    fn the_document_column_is_written_with_its_state() {
        assert!(
            INSERT_ANCHOR_SQL.contains("document_id, document_state"),
            "the document and its presence state must be written together: \
             {INSERT_ANCHOR_SQL}"
        );
    }

    /// The write returns the ledger row's id, so the caller can log which anchor
    /// it just wrote rather than reporting an unidentifiable success.
    #[test]
    fn the_insert_returns_the_anchor_id() {
        assert!(
            INSERT_ANCHOR_SQL.contains("RETURNING anchor_id"),
            "the caller needs the id to log what it wrote: {INSERT_ANCHOR_SQL}"
        );
    }
}
