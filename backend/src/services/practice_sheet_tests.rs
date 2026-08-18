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
    assert_eq!(heading(&s, 6, 2, 0, false), "6 questions. 2 to repeat.");
    assert_eq!(
        heading(&s, 5, 0, 0, false),
        "5 questions. Nothing to repeat."
    );
    assert!(!heading(&s, 5, 0, 0, false).contains('{'));
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
        false,
        &[],
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
        false,
        &[],
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
        false,
        &[],
    );

    assert!(payload.rows.is_empty());
    assert_eq!(payload.heading, "0 questions. Nothing to repeat.");
}

/// A `skipped` row is neither fine nor repeat, anywhere on the sheet.
///
/// The flow v1 migration widened the stored vocabulary to three values. Before
/// this test the reader was `if mark == "repeat" { repeat } else { fine }`, so a
/// question Marie had SET ASIDE printed on Chuck's sheet as **fine** — the sheet
/// telling him she answered something she had not. Silent, and caught by nothing:
/// no test passed a third value through.
#[test]
fn a_skipped_row_prints_as_skipped_and_is_counted_as_neither() {
    let s = settings();
    let payload = sheet_payload(
        &s,
        "S-5",
        Utc.with_ymd_and_hms(2026, 8, 18, 12, 0, 0).unwrap(),
        vec![
            row("george", None, Some(4), "fine", false),
            row("george", None, Some(2), "skipped", false),
            row("chuck", None, None, "repeat", false),
        ],
        false,
        &[],
    );

    let marks: Vec<&str> = payload.rows.iter().map(|r| r.mark.as_str()).collect();
    assert_eq!(marks, vec!["fine", "skipped", "repeat"]);

    // One to repeat, not two: a set-aside question is not a stumble.
    assert_eq!(payload.heading, "3 questions. 1 to repeat. 1 skipped.");
}

/// The two new clauses appear only when they are true, and in the drawn order.
#[test]
fn the_headline_gains_its_clauses_only_when_they_apply() {
    let s = settings();
    assert_eq!(
        heading(&s, 5, 0, 0, false),
        "5 questions. Nothing to repeat."
    );
    assert_eq!(
        heading(&s, 2, 0, 1, true),
        "2 questions. Nothing to repeat. 1 skipped. Ended early."
    );
    assert_eq!(
        heading(&s, 4, 2, 0, true),
        "4 questions. 2 to repeat. Ended early."
    );
    // No stored template leaks an unfilled placeholder into a printed sheet.
    assert!(!heading(&s, 2, 1, 1, true).contains('{'));
}

/// A mark the migration permits and this code has not learned is VISIBLE.
///
/// The old `else` arm disguised an unknown value as "fine". If a fourth mark is
/// ever added to the CHECK constraint without being added to `mark_cell`, the
/// sheet must show something a reader can question — not a pass.
#[test]
fn an_unknown_mark_renders_as_itself_rather_than_as_fine() {
    let s = settings();
    assert_eq!(mark_cell(&s, "deferred"), "deferred");
    assert_ne!(
        mark_cell(&s, "deferred"),
        s.practice_report_wording.mark_fine
    );
}

// ── The flag list at the foot of the sheet ──────────────────────────────────

fn flagged(side: &str, sort_order: i32, note: &str) -> FlaggedQuestionRecord {
    FlaggedQuestionRecord {
        side: side.to_string(),
        text: format!("question {sort_order}"),
        sort_order,
        flag_note: Some(note.to_string()),
    }
}

/// The label is a POSITION on its own side, not a row in the deck.
///
/// George's second question is G2 whether or not Chuck's are interleaved with
/// it in the deck's sort order. Roman reads `G2` on paper and goes to the second
/// George entry on the seed; a label that counted deck-wide would send him to
/// the wrong one on any mixed deck.
#[test]
fn the_flag_label_counts_within_a_side_and_not_across_the_deck() {
    let s = settings();
    let lines = flag_lines(
        &s,
        &[
            flagged("george", 1, "too soft"),
            flagged("chuck", 2, "leading"),
            flagged("george", 3, "wrong date"),
        ],
    );

    assert_eq!(lines.len(), 3);
    assert!(lines[0].starts_with("G1 —"), "{}", lines[0]);
    assert!(lines[1].starts_with("C1 —"), "{}", lines[1]);
    // The THIRD deck row, but George's SECOND question.
    assert!(lines[2].starts_with("G2 —"), "{}", lines[2]);
}

/// The line carries the question and the note, and leaks no placeholder.
#[test]
fn a_flag_line_prints_the_question_and_the_note() {
    let s = settings();
    let lines = flag_lines(&s, &[flagged("george", 1, "too soft")]);

    assert!(lines[0].contains("question 1"), "{}", lines[0]);
    assert!(lines[0].contains("too soft"), "{}", lines[0]);
    assert!(
        !lines[0].contains('{'),
        "an unfilled placeholder: {}",
        lines[0]
    );
}

/// Nothing flagged withdraws the whole block — heading and hint included.
///
/// A heading over an empty list reads as a list that failed to load, which on a
/// printed sheet is indistinguishable from one that was never written.
#[test]
fn an_unflagged_deck_withdraws_the_block_rather_than_printing_an_empty_heading() {
    let s = settings();
    assert!(flag_lines(&s, &[]).is_empty());

    let payload = sheet_payload(
        &s,
        "S-5",
        Utc.with_ymd_and_hms(2026, 8, 18, 12, 0, 0).unwrap(),
        vec![row("george", None, Some(4), "fine", false)],
        false,
        &[],
    );
    assert!(payload.flagged.is_empty());
    assert_eq!(payload.flagged_heading, "");
    assert_eq!(payload.flagged_hint, "");
}

/// A flagged deck carries the block's own words, from the store.
#[test]
fn a_flagged_deck_carries_the_blocks_heading_and_hint() {
    let s = settings();
    let payload = sheet_payload(
        &s,
        "S-5",
        Utc.with_ymd_and_hms(2026, 8, 18, 12, 0, 0).unwrap(),
        vec![row("george", None, Some(4), "fine", false)],
        false,
        &[flagged("george", 1, "too soft")],
    );

    assert_eq!(payload.flagged.len(), 1);
    assert_eq!(payload.flagged_heading, "Flagged before the session");
    assert!(!payload.flagged_hint.is_empty());
}
