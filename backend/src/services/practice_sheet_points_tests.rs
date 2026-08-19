// Tests for the sheet's `points_to` cell.
//
// Split from `practice_sheet_tests` on 2026-08-19 when Part B carried it past
// Rule 17's limit. The seam: the sibling is about the sheet's SHAPE — its
// heading, its From cell, its marks — and these three are about one cell whose
// stored value has three states (never opened, opened and empty, malformed) and
// exactly one rendering.

use super::*;
// The two fixtures the sibling already owns. `pub(super)` there reaches into
// `practice_sheet` and everything under it — exactly this module, and no
// further.
use super::tests::{row, settings};
use chrono::Utc;

/// The receipts she named ride on the row, in the order she picked them.
#[test]
fn the_sheet_carries_what_she_said_she_would_point_to() {
    let s = settings();
    let mut row = row("george", None, Some(4), "fine", false);
    row.points_to = Some(serde_json::json!([
        "your certified letter, 16 Nov 2009",
        "CFS Interrogatory Response, p. 10"
    ]));

    let payload = sheet_payload(
        &s,
        SheetSources {
            code: "S-5",
            ended_at: Utc::now(),
            rows: vec![row],
            ended_early: false,
            flagged: &[],
            changes: vec![],
        },
    );
    assert_eq!(
        payload.rows[0].points_to,
        vec![
            "your certified letter, 16 Nov 2009".to_string(),
            "CFS Interrogatory Response, p. 10".to_string()
        ]
    );
}

/// An answer that named nothing prints NOTHING, and so does one from before the
/// control existed.
///
/// Two different stored values — `Some([])` and `None` — and one rendering,
/// because the SHEET's question is "what did she point to" and the answer is
/// "nothing" either way. The distinction the column keeps is for the log, not
/// for Chuck's paper: a "would point to:" with an empty list after it reads as
/// data that went missing.
#[test]
fn an_answer_that_named_nothing_prints_no_line() {
    let s = settings();
    let mut never = row("george", None, Some(4), "fine", false);
    never.points_to = None;
    let mut empty = row("george", None, Some(4), "fine", false);
    empty.points_to = Some(serde_json::json!([]));

    let payload = sheet_payload(
        &s,
        SheetSources {
            code: "S-5",
            ended_at: Utc::now(),
            rows: vec![never, empty],
            ended_early: false,
            flagged: &[],
            changes: vec![],
        },
    );
    assert!(payload.rows[0].points_to.is_empty());
    assert!(payload.rows[1].points_to.is_empty());
}

/// A stored value that is not a list of strings withdraws the line rather than
/// failing the sheet.
///
/// Chuck's sheet is printed paper. Refusing to render it over one malformed cell
/// would cost him the whole sitting, and there is nothing about that cell he
/// needs more than the other six columns. The log names it; the paper says
/// nothing, which is honest.
#[test]
fn a_malformed_points_to_withdraws_the_line_and_never_fails_the_sheet() {
    let s = settings();
    let mut row = row("george", None, Some(4), "fine", false);
    row.points_to = Some(serde_json::json!({ "not": "a list" }));

    let payload = sheet_payload(
        &s,
        SheetSources {
            code: "S-5",
            ended_at: Utc::now(),
            rows: vec![row],
            ended_early: false,
            flagged: &[],
            changes: vec![],
        },
    );
    assert_eq!(payload.rows.len(), 1, "the sheet still renders");
    assert!(payload.rows[0].points_to.is_empty());
    assert_eq!(
        payload.rows[0].answer, "her answer",
        "every other cell stands"
    );
}
