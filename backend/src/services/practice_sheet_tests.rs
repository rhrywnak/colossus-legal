// Tests for `services::practice_sheet`.
//
// This sheet is PRINTED and handed to a lawyer. Every assertion below is about a
// cell he will read on paper, where there is no tooltip and no second click.

use super::*;
use chrono::{TimeZone, Utc};

fn settings() -> Settings {
    Settings::for_test()
}

fn row(
    side: &str,
    braid: Option<&str>,
    tactic: Option<i16>,
    mark: &str,
    help: bool,
) -> PracticeSheetRow {
    PracticeSheetRow {
        side: side.to_string(),
        braid_rows: braid.map(str::to_string),
        tactic,
        question: "a question".to_string(),
        answer_text: "her answer".to_string(),
        mark: mark.to_string(),
        help_opened: help,
    }
}

/// The three "From" cells, each from its own stored row.
#[test]
fn the_from_cell_distinguishes_cross_braid_and_direct() {
    let s = settings();
    assert_eq!(
        from_cell(&s, &row("george", None, Some(4), "fine", false)),
        "George"
    );
    assert_eq!(
        from_cell(
            &s,
            &row("george", Some("rows 1 · 2"), Some(5), "fine", false)
        ),
        "George · braid"
    );
    assert_eq!(
        from_cell(&s, &row("chuck", None, None, "fine", false)),
        "Chuck"
    );
}

/// The heading reads as a sentence at zero, and counts at one and above.
///
/// "0 to repeat." is the sentence nobody writes by hand, and this heading is the
/// first line Chuck reads. Getting it wrong costs the sheet its credibility
/// before he reaches a single answer.
#[test]
fn the_heading_reads_as_a_sentence_whether_or_not_anything_needs_repeating() {
    let s = settings();
    assert_eq!(heading(&s, 6, 2), "6 questions. 2 to repeat.");
    assert_eq!(heading(&s, 5, 0), "5 questions. Nothing to repeat.");
    assert!(!heading(&s, 5, 0).contains('{'));
}

/// Every cell arrives as a word, and the numbering starts at one.
#[test]
fn every_cell_arrives_as_a_word_a_lawyer_can_read_on_paper() {
    let s = settings();
    let payload = sheet_payload(
        &s,
        "S-5",
        Utc.with_ymd_and_hms(2026, 8, 17, 20, 0, 0).unwrap(),
        vec![
            row("george", None, Some(4), "repeat", true),
            row("chuck", None, None, "fine", false),
        ],
    );

    assert_eq!(payload.kicker, "Session done · S-5 · Mon 17 Aug");
    assert_eq!(payload.heading, "2 questions. 1 to repeat.");

    let first = &payload.rows[0];
    assert_eq!(first.number, 1);
    assert_eq!(first.from, "George");
    assert_eq!(first.tactic, "false premise");
    assert_eq!(first.mark, "repeat");
    assert_eq!(first.help, "opened");
    assert!(first.help_opened, "the flag rides along for the emphasis");

    // A question with no card gets the stored dash — never an empty cell, which
    // on paper reads as data that went missing rather than as "none".
    let second = &payload.rows[1];
    assert_eq!(second.number, 2);
    assert_eq!(second.tactic, "—");
    assert_eq!(second.mark, "fine");
    assert_eq!(second.help, "—");
    assert!(!second.help_opened);
}

/// Her words are printed verbatim — never trimmed, summarised or re-cased.
///
/// The sheet's whole evidentiary value is that it is what she actually typed. A
/// helpful normalisation here would quietly change the record Chuck runs his mock
/// cross from.
#[test]
fn her_answer_is_printed_exactly_as_she_typed_it() {
    let s = settings();
    let mut r = row("george", None, Some(4), "fine", false);
    r.answer_text = "  Well, we did argue, we were at each other's throats a bit.  ".to_string();

    let payload = sheet_payload(
        &s,
        "S-5",
        Utc.with_ymd_and_hms(2026, 8, 17, 20, 0, 0).unwrap(),
        vec![r.clone()],
    );
    assert_eq!(payload.rows[0].answer, r.answer_text);
}

/// An empty session still composes a sheet.
///
/// She can end a session having answered nothing. The sheet must then say "0
/// questions. Nothing to repeat." rather than failing — a blank screen at the end
/// of a session reads as work that was lost.
#[test]
fn a_session_with_no_answers_still_composes_a_sheet() {
    let s = settings();
    let payload = sheet_payload(
        &s,
        "S-5",
        Utc.with_ymd_and_hms(2026, 8, 17, 20, 0, 0).unwrap(),
        vec![],
    );

    assert!(payload.rows.is_empty());
    assert_eq!(payload.heading, "0 questions. Nothing to repeat.");
}
