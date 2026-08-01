//! Allocation tests for the scenario code (`S-n`) — §2a, tracker task 1.1.
//!
//! Two kinds of assertion, neither of which needs a live database:
//!
//! * SQL-shape guards on [`super::INSERT_SCENARIO_SQL`], pinning the clauses that
//!   PRODUCE atomic, never-reused allocation (the house pattern used by
//!   `scenario_candidate_ordinals` and the merge/reconcile guards);
//! * disk/code guards reading the migration itself, pinning the constraints the
//!   allocation relies on (Standing Rule 21).
//!
//! ## Why these matter more than they look
//!
//! Never-reuse is not observable in a passing build. Nothing breaks, no test goes
//! red, and no error is logged when a code is reissued — a human simply finds that
//! the "S-7" in their rehearsal note now names a different scenario. The failure
//! surfaces months later as confusion in a room, which is the worst possible place
//! for it. So the properties that prevent it are asserted at the only point where
//! they are visible: the text of the statement and the DDL.

use std::path::PathBuf;

use super::INSERT_SCENARIO_SQL;

/// The migration that introduced the code column and its sequence table.
const CODES_MIGRATION: &str = "20260801103312_add_scenario_codes.sql";

/// Read a pipeline migration file by name from disk.
fn read_migration(filename: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("pipeline_migrations")
        .join(filename);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read migration {}: {e}", path.display()))
}

// ── The allocation statement ──────────────────────────────────────────────────

/// Allocation and insertion are ONE statement.
///
/// If these ever became two statements without an explicit transaction, a crash
/// between them would either spend a code on no scenario (a harmless hole) or —
/// far worse, if ordered the other way — create a scenario with no code, which the
/// NOT NULL would reject and turn into a failed creation the human cannot explain.
#[test]
fn allocation_and_insertion_are_a_single_atomic_statement() {
    assert!(
        INSERT_SCENARIO_SQL.contains("WITH bumped AS ("),
        "the sequence bump must be a data-modifying CTE of the insert: {INSERT_SCENARIO_SQL}"
    );
    // One statement means exactly one terminator's worth of INSERTs feeding each
    // other — the CTE's, and the scenario's.
    assert_eq!(
        INSERT_SCENARIO_SQL.matches("INSERT INTO").count(),
        2,
        "expected exactly two INSERTs (the sequence bump and the scenario): \
         {INSERT_SCENARIO_SQL}"
    );
    assert!(
        !INSERT_SCENARIO_SQL.contains(';'),
        "a semicolon would mean two statements and lose the atomicity: {INSERT_SCENARIO_SQL}"
    );
}

/// The next code comes from the SEQUENCE, never from the scenarios table.
///
/// This is THE never-reuse assertion. `MAX(code_ordinal)` over `scenarios` would
/// look correct and pass every test that does not delete a scenario — then rewind
/// the moment the highest-coded scenario is deleted, reissuing a code that
/// scenario already wore.
#[test]
fn the_next_code_never_comes_from_a_max_over_live_scenarios() {
    assert!(
        INSERT_SCENARIO_SQL.contains("case_code_sequences.next_ordinal + 1"),
        "the next code must continue the stored sequence: {INSERT_SCENARIO_SQL}"
    );
    assert!(
        !INSERT_SCENARIO_SQL.contains("MAX("),
        "allocation must NEVER read a MAX over live rows — that rewinds when the \
         highest-coded scenario is deleted and reissues its code: {INSERT_SCENARIO_SQL}"
    );
}

/// A case's first scenario starts at 1, and the sequence is per case.
#[test]
fn the_first_scenario_of_a_case_is_s_one() {
    // The seed branch of the upsert supplies 1 directly; the conflict branch adds
    // one to what is stored. Both are present, so an unseen case starts at S-1
    // rather than S-0 or a NULL.
    assert!(
        INSERT_SCENARIO_SQL.contains("VALUES ($4, 1)"),
        "a case with no sequence row yet must start at 1: {INSERT_SCENARIO_SQL}"
    );
    assert!(
        INSERT_SCENARIO_SQL.contains("ON CONFLICT (case_slug) DO UPDATE"),
        "the sequence must be per case, keyed on the slug: {INSERT_SCENARIO_SQL}"
    );
}

/// The caller gets the code back, so it can be logged and returned without a
/// second read.
#[test]
fn the_insert_returns_both_the_id_and_the_code() {
    assert!(
        INSERT_SCENARIO_SQL.contains("RETURNING scenario_id, code_ordinal"),
        "the caller needs both to answer without a read-back: {INSERT_SCENARIO_SQL}"
    );
}

// ── The migration's guarantees ────────────────────────────────────────────────

/// The database refuses two scenarios sharing a code within a case.
///
/// The loud backstop: if allocation is ever bypassed (a manual insert, a future
/// code path that forgets the CTE) the second write fails rather than minting an
/// ambiguous handle.
#[test]
fn the_migration_makes_a_duplicate_code_impossible() {
    let sql = read_migration(CODES_MIGRATION);
    assert!(
        sql.contains("UNIQUE (case_slug, code_ordinal)"),
        "a duplicate S-n within a case must be refused by the database"
    );
    assert!(
        sql.contains("ALTER COLUMN code_ordinal SET NOT NULL"),
        "every scenario must carry a code once the backfill has run"
    );
}

/// The backfill assigns codes in creation order, deterministically.
///
/// Creation order is what a human already saw, so S-1 is the case's oldest
/// scenario. The `scenario_id` tie-break matters: without it, two rows sharing a
/// `created_at` could be numbered either way on two different databases, and DEV
/// and PROD would disagree about which scenario is S-4.
#[test]
fn the_backfill_is_ordered_and_deterministic() {
    let sql = read_migration(CODES_MIGRATION);
    assert!(
        sql.contains("PARTITION BY case_slug ORDER BY created_at, scenario_id"),
        "the backfill must number per case, oldest first, with a stable tie-break"
    );
}

/// The sequence is seeded in the SAME migration that backfills.
///
/// The window this closes is small and fatal: between a backfill and a separate
/// seeding, the sequence still reads 0, so a backend that started in that window
/// would mint S-1 for a case that already has one and hit the unique violation —
/// or, before the constraint existed, silently duplicate it.
#[test]
fn the_sequence_is_seeded_from_the_backfill_in_the_same_migration() {
    let sql = read_migration(CODES_MIGRATION);
    assert!(
        sql.contains("INSERT INTO case_code_sequences"),
        "the sequence must be seeded by this migration"
    );
    assert!(
        sql.contains("MAX(code_ordinal)"),
        "the seed must be the highest code the backfill assigned"
    );
    // Seeding must come AFTER the backfill in file order, or it reads zeros.
    let backfill_at = sql
        .find("SET code_ordinal = ordered.assigned_ordinal")
        .expect("the backfill UPDATE must be present");
    let seed_at = sql
        .find("INSERT INTO case_code_sequences")
        .expect("the seed INSERT must be present");
    assert!(
        backfill_at < seed_at,
        "the sequence must be seeded AFTER the backfill assigns codes, or it seeds 0"
    );
}

/// The sequence table cannot rewind below zero.
#[test]
fn the_sequence_cannot_go_negative() {
    let sql = read_migration(CODES_MIGRATION);
    assert!(
        sql.contains("next_ordinal >= 0"),
        "a negative sequence would mean never-reuse had already failed"
    );
}
