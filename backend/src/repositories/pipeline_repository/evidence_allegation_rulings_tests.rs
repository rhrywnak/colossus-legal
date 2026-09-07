//! Tests for `evidence_allegation_rulings` (PROOF_MATRIX_v2 §2).
//!
//! These pin the SQL's SHAPE rather than its behaviour against a live database,
//! matching `evidence_allegation_links_tests`: the properties that matter here —
//! case-wide keying, upsert rather than duplicate, `ruled_at` frozen across a
//! reversal, the ledger's verdict being NULL on a withdrawal and only there — are
//! all readable from the statements, and reading them needs no Postgres.
//!
//! The behavioural half (a real upsert against a real table, both writes in one
//! transaction) is `rulings_live` in `tests/proof_matrix_rulings_live.rs`, which
//! runs `--ignored` against a scratch database.

use super::*;

/// Tokens of the `SET` clause on the upsert's conflict branch.
///
/// Parsed rather than matched literally, for the reason the sibling module
/// records: an assertion against the exact string breaks the day a column is
/// added, and breaks by reporting the wrong invariant as violated.
fn conflict_set_columns() -> Vec<String> {
    let start = UPSERT_RULING_SQL
        .find("DO UPDATE SET")
        .expect("the upsert has a conflict branch");
    UPSERT_RULING_SQL[start + "DO UPDATE SET".len()..]
        .split(',')
        .filter_map(|clause| clause.split('=').next())
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect()
}

// ─── The key is the pair, and the scope is the case ──────────────────────────

/// THE CASE-WIDE TEST. A ruling belongs to the item, not to a Count or a page.
///
/// The same allegation is reached from several Counts through several Elements. A
/// `count_number` or a `scenario_id` in this key would make one item kept under
/// Count 1 and unruled under Count 4 — two answers to one question, on a surface
/// whose entire value is that it has one. It would also compile and pass every
/// other test here, which is why this one is written down.
#[test]
fn a_ruling_is_keyed_by_the_item_and_the_accusation_alone() {
    assert!(
        UPSERT_RULING_SQL.contains("ON CONFLICT (evidence_id, allegation_id)"),
        "the conflict target must be the pair: {UPSERT_RULING_SQL}"
    );
    for forbidden in ["scenario_id", "count_number", "element_id"] {
        assert!(
            !UPSERT_RULING_SQL.contains(forbidden),
            "a {forbidden} column here makes a ruling mean different things on \
             different pages: {UPSERT_RULING_SQL}"
        );
        assert!(
            !RULING_COLUMNS.contains(forbidden),
            "the projection names {forbidden}, so the table has grown it: \
             {RULING_COLUMNS}"
        );
        assert!(
            !INSERT_RULING_EVENT_SQL.contains(forbidden),
            "the ledger must be case-wide too: {INSERT_RULING_EVENT_SQL}"
        );
    }
}

// ─── The upsert's shape ──────────────────────────────────────────────────────

/// Ruling the same pair again reverses it; it never stores a second row.
///
/// Two rows for one pair would be an item both kept and removed under the same
/// paragraph — a contradiction the ordering would then have to render, and it has
/// no way to.
#[test]
fn re_ruling_a_pair_updates_it_rather_than_duplicating_it() {
    assert!(UPSERT_RULING_SQL.contains("INSERT INTO evidence_allegation_rulings"));
    assert!(UPSERT_RULING_SQL.contains("DO UPDATE SET"));

    let updated = conflict_set_columns();
    for column in ["ruling", "ruled_by", "updated_at", "note"] {
        assert!(
            updated.iter().any(|c| c == column),
            "a reversal must change {column}: {updated:?}"
        );
    }
}

/// `ruled_at` survives a reversal; `updated_at` moves.
///
/// The two dates answer different questions — when did a human FIRST go through
/// this item, and when did they last change their mind. A conflict branch that
/// touched `ruled_at` would make every item look freshly reviewed the moment one
/// of them was reversed.
#[test]
fn a_reversal_keeps_the_original_ruled_at() {
    let updated = conflict_set_columns();
    assert!(
        !updated.iter().any(|c| c == "ruled_at"),
        "ruled_at must not be touched on conflict: {updated:?}"
    );
    assert!(
        updated.iter().any(|c| c == "updated_at"),
        "updated_at must move on conflict: {updated:?}"
    );
}

/// The upsert reports whether it inserted or updated, in the same statement.
///
/// `xmax <> 0` is what lets the caller write `rule` vs `rerule` into the ledger
/// without a preceding SELECT another request could interleave with. Without it
/// the two acts would be indistinguishable in the record — and the record is the
/// only thing that can say a human changed their mind.
#[test]
fn the_upsert_reports_whether_the_row_already_existed() {
    assert!(
        UPSERT_RULING_SQL.contains("RETURNING (xmax <> 0) AS existed"),
        "the upsert must report insert-vs-update: {UPSERT_RULING_SQL}"
    );
}

/// Both timestamps are bound from ONE parameter on insert.
///
/// `VALUES (…, $5, $5, …)` is what makes a freshly ruled item's two dates equal.
/// Two separate binds would let them differ by a round trip, and "ruled at
/// 12:00:00.001, last updated 12:00:00.004" reads as a reversal that never
/// happened.
#[test]
fn a_new_ruling_gets_one_timestamp_for_both_columns() {
    assert!(
        UPSERT_RULING_SQL.contains("VALUES ($1, $2, $3, $4, $5, $5, $6)"),
        "ruled_at and updated_at must share a bind on insert: {UPSERT_RULING_SQL}"
    );
}

// ─── The ledger's shape ──────────────────────────────────────────────────────

/// The ledger records the act, the verdict, the note, the actor and the time.
///
/// Every one of those is load-bearing: an event with no actor attributes nothing,
/// and an event with no action cannot tell a reversal from a first reading.
#[test]
fn the_ledger_records_who_did_what_and_when() {
    for column in [
        "evidence_id",
        "allegation_id",
        "action",
        "ruling",
        "note",
        "actor",
        "at",
    ] {
        assert!(
            INSERT_RULING_EVENT_SQL.contains(column),
            "the ledger must carry {column}: {INSERT_RULING_EVENT_SQL}"
        );
    }
}

/// The ledger is append-only: it is never updated and never deleted from.
///
/// A ledger that can be edited is a record nobody can rely on, and this one is
/// what says a human ever looked at an item that has since been withdrawn.
#[test]
fn nothing_in_this_module_updates_or_deletes_the_ledger() {
    let source = include_str!("evidence_allegation_rulings.rs");
    assert!(
        !source.contains("UPDATE evidence_allegation_ruling_events"),
        "the ledger must never be updated"
    );
    assert!(
        !source.contains("DELETE FROM evidence_allegation_ruling_events"),
        "the ledger must never be deleted from"
    );
}

/// The state table IS deleted from — that is what a withdrawal is — and the
/// delete is keyed by the pair, never by the item alone.
///
/// A delete keyed on `evidence_id` alone would withdraw a human's verdict on
/// every OTHER accusation the same statement bears on, from one click on one row.
#[test]
fn a_withdrawal_deletes_exactly_one_pair() {
    let source = include_str!("evidence_allegation_rulings.rs");
    let at = source
        .find("DELETE FROM evidence_allegation_rulings")
        .expect("the withdrawal deletes the state row");
    let statement = &source[at..at + 160];
    assert!(
        statement.contains("evidence_id = $1") && statement.contains("allegation_id = $2"),
        "the delete must name both halves of the key: {statement}"
    );
}

// ─── The read boundary ───────────────────────────────────────────────────────

/// A stored token this build knows becomes a typed verdict.
#[test]
fn a_known_verdict_token_parses() {
    let record = record_with("keep");
    assert_eq!(
        record.verdict().expect("'keep' is a verdict"),
        MatrixRuling::Keep
    );
}

/// A stored token this build does NOT know is refused, and the refusal names the
/// item — not just the column.
///
/// This is the whole argument for keeping `ruling` a `String` in `FromRow`. sqlx
/// would have said "could not decode column 'ruling'"; an operator would then
/// have to find which of a thousand rows it meant.
#[test]
fn an_unreadable_verdict_token_is_refused_and_names_the_item() {
    let record = record_with("delete");
    let err = record.verdict().expect_err("'delete' is not a verdict");
    let message = err.to_string();
    assert!(
        message.contains("evidence-074"),
        "names the item: {message}"
    );
    assert!(
        message.contains("allegation-41"),
        "names the accusation: {message}"
    );
    assert!(
        message.contains("delete"),
        "names the offending token: {message}"
    );
}

/// The projection carries every column `RulingRecord` decodes.
///
/// The derive maps BY NAME, so a column dropped from the projection is a runtime
/// decode failure at the first read — on a live page, not in CI.
#[test]
fn the_projection_covers_every_decoded_field() {
    for column in [
        "evidence_id",
        "allegation_id",
        "ruling",
        "ruled_by",
        "ruled_at",
        "updated_at",
        "note",
    ] {
        assert!(
            RULING_COLUMNS.contains(column),
            "the projection is missing {column}: {RULING_COLUMNS}"
        );
    }
}

/// A `RulingRecord` carrying an arbitrary token, for the read-boundary tests.
fn record_with(token: &str) -> RulingRecord {
    let at = DateTime::<Utc>::from_timestamp(1_757_000_000, 0).expect("a valid instant");
    RulingRecord {
        evidence_id: "evidence-074".to_string(),
        allegation_id: "allegation-41".to_string(),
        ruling: token.to_string(),
        ruled_by: "roman".to_string(),
        ruled_at: at,
        updated_at: at,
        note: None,
    }
}
