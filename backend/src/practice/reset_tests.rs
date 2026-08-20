//! What the reset REPORTS, asserted without a database.
//!
//! The deletes need a live pool and are covered the way this repository covers
//! pool-bound writes: their SQL is parsed off disk by `practice_sql_shape` and
//! checked against the migrations. What is testable here is the part an operator
//! actually reads before typing `--apply` — the proof — and the emptiness rule
//! the binary branches on.

use super::{render_report, ResetCounts};

fn counts(sessions: i64, answers: i64, notes: i64) -> ResetCounts {
    ResetCounts {
        sessions,
        answers,
        notes,
    }
}

/// Nothing to do is all three at zero — not "no sessions".
///
/// A scenario can carry notes with no sitting (Chuck writing on the deck before
/// Marie ever practises). Treating "no sessions" as empty would report nothing
/// to clear while notes sat there waiting to be surprised by an --apply.
#[test]
fn empty_means_all_three_are_zero() {
    assert!(counts(0, 0, 0).is_empty());
    assert!(!counts(0, 0, 1).is_empty(), "a note alone is not nothing");
    assert!(!counts(0, 1, 0).is_empty());
    assert!(!counts(1, 0, 0).is_empty());
}

/// A dry run says so, and says nothing was written.
#[test]
fn the_dry_run_report_says_nothing_was_written() {
    let out = render_report("S-6", &counts(3, 27, 4), None);
    assert!(out.contains("DRY RUN"), "{out}");
    assert!(out.contains("nothing was written"), "{out}");
    assert!(out.contains("--apply"), "the way forward is named: {out}");
    // The counts it WOULD clear are the proof an operator reads before writing.
    assert!(out.contains("3 -> 0 (would be)"), "{out}");
    assert!(out.contains("27 -> 0 (would be)"), "{out}");
}

/// An applied run shows before -> after for each table.
#[test]
fn the_applied_report_shows_before_and_after() {
    let out = render_report("S-6", &counts(3, 27, 4), Some(&counts(0, 0, 0)));
    assert!(out.contains("APPLIED"), "{out}");
    assert!(!out.contains("DRY RUN"), "{out}");
    assert!(out.contains("3 -> 0"), "{out}");
    assert!(out.contains("27 -> 0"), "{out}");
    assert!(out.contains("4 -> 0"), "{out}");
}

/// The report names the two tables it did NOT touch.
///
/// This is the assertion that matters most on a witness's practice record: an
/// operator about to clear Marie's answers needs to read, on the same page,
/// that the deck Chuck wrote and the log of who edited it are not going
/// anywhere. Leaving that to trust is how a tool gets run once and never again.
#[test]
fn the_report_names_what_it_keeps() {
    let out = render_report("S-6", &counts(3, 27, 4), Some(&counts(0, 0, 0)));
    assert!(out.contains("kept, untouched"), "{out}");
    assert!(out.contains("practice_questions"), "{out}");
    assert!(out.contains("practice_deck_changes"), "{out}");
}

/// The scenario code appears, so a report file cannot be mistaken for another's.
#[test]
fn the_report_names_the_scenario() {
    assert!(render_report("S-6", &counts(0, 0, 0), None).contains("S-6"));
    assert!(render_report("S-5", &counts(0, 0, 0), None).contains("S-5"));
}

/// An after-count that is NOT zero still prints, rather than being hidden.
///
/// It should be impossible — the deletes and the count share a transaction — but
/// a report that silently rendered "0" regardless would be the one thing able to
/// hide a failed reset. It prints what it measured.
#[test]
fn a_nonzero_after_count_is_printed_not_hidden() {
    let out = render_report("S-6", &counts(3, 27, 4), Some(&counts(0, 2, 0)));
    assert!(
        out.contains("27 -> 2"),
        "an incomplete reset must be visible: {out}"
    );
}

// ─── The refusals, in the words an operator reads ───────────────────────────
//
// Both variants are constructable without a pool, so the SENTENCE is testable
// even though the query that raises it is not. What is pinned here is not the
// prose but the two things a person needs out of it: which scenario, and what to
// do next. A message that lost the code would send somebody to the wrong deck.

/// An unknown code names the code, and says what to do about it.
#[test]
fn an_unknown_scenario_names_the_code_and_the_way_forward() {
    let error = super::ResetError::UnknownScenario {
        code: "S-99".to_string(),
    };
    let said = error.to_string();
    assert!(
        said.contains("S-99"),
        "the offending code must appear: {said}"
    );
    assert!(
        said.contains("check the code"),
        "a refusal an operator can act on names the next step: {said}"
    );
}

/// A database refusal names the scenario AND the underlying cause.
///
/// The `#[source]` chain is what carries the Postgres error; the Display string
/// interpolates it, so one line in a terminal answers both "which scenario" and
/// "why" without an operator having to widen the log.
#[test]
fn a_database_refusal_names_the_scenario_and_the_cause() {
    let error = super::ResetError::Database {
        code: "S-6".to_string(),
        source: sqlx::Error::RowNotFound,
    };
    let said = error.to_string();
    assert!(said.contains("S-6"), "the scenario must appear: {said}");
    assert!(
        said.contains(&sqlx::Error::RowNotFound.to_string()),
        "the underlying cause must survive into the message: {said}"
    );
}
