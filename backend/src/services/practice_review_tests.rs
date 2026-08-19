// Tests for `services::practice_review`.
//
// The one arithmetic on this page that can be wrong in a way nothing else
// catches: attempt NUMBERING against attempt ORDER. The page says "newest
// first" out loud, and "attempt 1" must still be her first attempt — number
// from the top of a reversed list and attempt 1 changes its meaning every time
// she answers again, which would make Chuck's note on "attempt 1" point at a
// different answer each week.

use super::*;
use crate::domain::settings::Settings;
use crate::repositories::pipeline_repository::practice_notes::{AttemptRecord, NoteRecord};
use chrono::TimeZone;
use uuid::Uuid;

fn at(day: u32, hour: u32, minute: u32) -> chrono::DateTime<chrono::Utc> {
    chrono::Utc
        .with_ymd_and_hms(2026, 8, day, hour, minute, 0)
        .single()
        .expect("a real instant")
}

fn attempt(n: u128, day: u32, mark: &str) -> AttemptRecord {
    AttemptRecord {
        id: Uuid::from_u128(n),
        question_text: "Weren't you at each other's throats?".to_string(),
        answer_text: format!("answer {n}"),
        answered_at: at(day, 8, 40),
        mark: mark.to_string(),
        read_text: Some("Fine. Short, and yours.".to_string()),
        read_ok: Some(true),
        self_check: serde_json::json!({
            "only_asked": true, "accepted_premise": false,
            "explained_unasked": false, "guessed": false
        }),
        help_opened: false,
        points_to: None,
    }
}

fn note_on(answer: u128) -> NoteRecord {
    NoteRecord {
        id: Uuid::from_u128(900 + answer),
        question_id: Some(Uuid::from_u128(1)),
        answer_id: Some(Uuid::from_u128(answer)),
        author: "Chuck".to_string(),
        text: "Letter, date, stop.".to_string(),
        created_at: at(18, 12, 0),
        struck_at: None,
        struck_by: None,
    }
}

/// Numbered from her FIRST attempt, and returned newest first.
#[test]
fn attempts_are_numbered_from_the_first_and_returned_newest_first() {
    let s = Settings::for_test();
    let rows = vec![attempt(1, 18, "repeat"), attempt(2, 19, "fine")];

    let out = attempts(&s, &rows, &[]);

    assert_eq!(out.len(), 2);
    assert_eq!(out[0].answer_id, Uuid::from_u128(2), "newest first");
    assert!(
        out[0].heading.starts_with("attempt 2"),
        "{}",
        out[0].heading
    );
    assert!(
        out[1].heading.starts_with("attempt 1"),
        "{}",
        out[1].heading
    );
}

/// The heading carries the day AND the clock.
#[test]
fn an_attempt_heading_carries_the_day_and_the_clock() {
    let s = Settings::for_test();
    let out = attempts(&s, &[attempt(1, 19, "fine")], &[]);
    assert_eq!(out[0].heading, "attempt 1 · Wed 19 Aug 08:40");
    assert!(!out[0].heading.contains('{'), "a placeholder survived");
}

/// The mark arrives as BOTH the stored word and the raw key.
///
/// The word is what a person reads; the key is what the screen colours by. A
/// screen matching on the sentence would lose its colours the first time
/// somebody edited the wording row.
#[test]
fn the_mark_arrives_as_the_stored_word_and_the_raw_key() {
    let s = Settings::for_test();
    let out = attempts(&s, &[attempt(1, 19, "repeat")], &[]);
    assert_eq!(out[0].mark, "repeat");
    assert_eq!(out[0].mark_key, "repeat");

    let skipped = attempts(&s, &[attempt(1, 19, "skipped")], &[]);
    assert_eq!(skipped[0].mark, "skipped", "and never 'fine'");
}

/// The detail line names the boxes she ticked, by their stored labels.
#[test]
fn the_detail_line_names_the_boxes_she_ticked() {
    let s = Settings::for_test();
    let out = attempts(&s, &[attempt(1, 19, "fine")], &[]);
    assert!(
        out[0]
            .detail
            .contains("I answered only the question that was asked"),
        "{}",
        out[0].detail
    );
    assert!(out[0].detail.starts_with("help: —"), "{}", out[0].detail);
}

/// Ticking NOTHING is a named absence, not an empty clause and not a fault.
#[test]
fn ticking_no_boxes_reads_as_a_named_absence() {
    let s = Settings::for_test();
    let mut row = attempt(1, 19, "fine");
    row.self_check = serde_json::json!({
        "only_asked": false, "accepted_premise": false,
        "explained_unasked": false, "guessed": false
    });

    let out = attempts(&s, &[row], &[]);
    assert!(out[0].detail.ends_with("none ticked"), "{}", out[0].detail);
}

/// A malformed `points_to` withdraws the clause and never fails the page.
#[test]
fn a_malformed_points_to_withdraws_the_clause() {
    let s = Settings::for_test();
    let mut row = attempt(1, 19, "fine");
    row.points_to = Some(serde_json::json!({ "not": "a list" }));

    let out = attempts(&s, &[row], &[]);
    assert_eq!(out.len(), 1, "the page still renders");
    assert!(out[0].points_to.is_empty());
    assert_eq!(out[0].answer, "answer 1", "every other cell stands");
}

/// A note lands on the attempt it names, and on no other.
#[test]
fn a_note_lands_on_the_attempt_it_names() {
    let s = Settings::for_test();
    let rows = vec![attempt(1, 18, "repeat"), attempt(2, 19, "fine")];

    let out = attempts(&s, &rows, &[note_on(1)]);

    // out[0] is attempt 2 (newest first); the note is on attempt 1.
    assert!(out[0].notes.is_empty());
    assert_eq!(out[1].notes.len(), 1);
    assert_eq!(out[1].notes[0].text, "Letter, date, stop.");
}

/// The QUESTION panel takes notes that name the question and no attempt.
///
/// Roman's amendment 2. A note on an attempt already renders under that
/// attempt; showing it here as well would say Chuck wrote it twice.
#[test]
fn the_question_panel_excludes_the_attempt_notes() {
    let s = Settings::for_test();
    let mut on_question = note_on(1);
    on_question.id = Uuid::from_u128(800);
    on_question.answer_id = None;

    let shown = question_notes(&s, &[note_on(1), on_question], Uuid::from_u128(1));

    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].id, Uuid::from_u128(800));
}

/// The progress line names the question's printed position.
#[test]
fn the_progress_line_names_the_printed_position() {
    assert_eq!(progress(&Settings::for_test(), 3), "Question 3 · review");
}
