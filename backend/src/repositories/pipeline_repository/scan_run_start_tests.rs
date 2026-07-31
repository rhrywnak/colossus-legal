//! Unit tests for the `scan_runs.rs` STUB/PROMOTE pair — the "failures leave
//! records" half of the run lifecycle.
//!
//! A second sibling test file rather than more of `scan_runs_tests.rs`: that file
//! reached the module-size limit, and these tests are a distinct subject (the
//! birth-and-promotion contract, not the INSERT's column shape). Both are wired
//! into `scan_runs.rs` with `#[cfg(test)] #[path = "…"] mod`.
//!
//! There is no live-DB harness in this repo, so — like the SQL-shape tests next
//! door — these read the shipped source and assert textual properties of the SQL
//! and of which status constant each statement binds. The live-database proof
//! lives in `backend/tests/scan_run_history_integration.rs` (`#[ignore]`d).

use super::tests::{columns_and_values, fn_body, scan_runs_source};

/// The stub INSERT is born FAILED, carries a reason, and leaves the progress
/// denominator NULL.
///
/// Three separate promises, all load-bearing:
///   * `failed`, not `running` — a row born `running` that never gets promoted is
///     indistinguishable from a live scan until the next restart sweeps it, which
///     is the blindness this change exists to remove.
///   * an `error` from birth — a failed row with no reason sends the reader back
///     to a log they may no longer have.
///   * `candidates_total` NULL — the pool has not been read, and "not known" must
///     not be written as `0`, which would claim an empty pool (Standing Rule 1).
#[test]
fn stub_insert_is_born_failed_with_a_reason_and_no_candidate_total() {
    let (columns, values) = columns_and_values();
    let pairs: Vec<(&str, &str)> = columns
        .iter()
        .map(String::as_str)
        .zip(values.iter().map(String::as_str))
        .collect();

    let value_for = |col: &str| -> &str {
        pairs
            .iter()
            .find(|(c, _)| *c == col)
            .map(|(_, v)| *v)
            .unwrap_or_else(|| panic!("the stub INSERT writes `{col}`; columns: {columns:?}"))
    };

    assert!(
        value_for("status").starts_with('$'),
        "status must be a bound placeholder, got `{}`",
        value_for("status")
    );
    assert!(
        value_for("error").starts_with('$'),
        "the stub's error reason must be a bound placeholder, got `{}`",
        value_for("error")
    );
    assert_eq!(
        value_for("candidates_total"),
        "NULL",
        "candidates_total must be NULL (not yet known), never a fabricated count"
    );

    let body = fn_body("insert_scan_run_stub");
    assert!(
        body.contains(".bind(SCAN_STATUS_FAILED)"),
        "the stub row must be born FAILED — bind SCAN_STATUS_FAILED, not another status"
    );
    assert!(
        !body.contains(".bind(SCAN_STATUS_RUNNING)"),
        "a stub born RUNNING re-opens the blind spot: an unpromoted row would look live"
    );
    assert!(
        body.contains(".bind(STUB_ERROR_UNFINISHED)"),
        "the stub must carry a reason from birth, not a NULL error"
    );
}

/// The promote UPDATE flips the row to `running`, CLEARS the stub's error, binds
/// the params snapshot as a placeholder, and only applies to a row still in the
/// birth state.
///
/// The `error = NULL` assignment is the one most likely to be dropped by a future
/// edit and the one whose absence is most misleading: the run would then display
/// as running/completed while still showing the stub's failure text. The
/// `AND status = $8` guard keeps a late promotion from resurrecting a run that
/// something else already failed.
#[test]
fn promote_update_runs_the_row_clears_the_error_and_guards_the_birth_state() {
    let text = scan_runs_source();
    // ANCHOR: `candidates_total = $` appears in the promote UPDATE and nowhere
    // else in this file (finalize does not touch the denominator). Same anchoring
    // discipline as `finalize_update_assigns_summary_json_a_placeholder`.
    let anchor = text
        .find("candidates_total = $")
        .expect("the promote UPDATE assigns candidates_total via a placeholder");
    let stmt_start = text[..anchor]
        .rfind("UPDATE scan_runs SET")
        .expect("the promote UPDATE opens with the SET clause");
    // ASSUMES a one-hash raw string, `r#"…"#` — see the note on `start_insert_sql`.
    let end = text[stmt_start..]
        .find("\"#")
        .expect("raw string terminates")
        + stmt_start;
    let sql = &text[stmt_start..end];

    assert!(
        sql.contains("status = $2"),
        "promotion must set the status from a bound placeholder, got: {sql}"
    );
    assert!(
        sql.contains("resolved_params = $4"),
        "resolved_params is jsonb and must be bound, never inlined, got: {sql}"
    );
    assert!(
        sql.contains("error = NULL"),
        "promotion MUST clear the stub's birth error, or a running run displays a \
         failure that never happened, got: {sql}"
    );
    assert!(
        sql.contains("WHERE run_id = $1 AND status = $8"),
        "promotion must be fenced to the run AND to the birth state, got: {sql}"
    );

    let body = fn_body("promote_scan_run_running");
    assert!(
        body.contains(".bind(SCAN_STATUS_RUNNING)") && body.contains(".bind(SCAN_STATUS_FAILED)"),
        "promotion binds the new status (running) and the guarded old one (failed)"
    );
}
