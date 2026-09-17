// Tests for `services::practice_note_view`.
//
// What a reader of a note must be able to trust: that a withdrawn note SAYS it
// was withdrawn, that no raw `{when}` reaches the screen, and that a deck row
// shows only the notes about what Marie would say today.

use chrono::{DateTime, TimeZone, Utc};

use super::*;

fn settings() -> Settings {
    let mut s = Settings::for_test();
    // A fixed zone, so the day asserted below cannot move with the machine.
    s.practice_read.case_timezone = "America/Detroit".to_string();
    s
}

fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, day, hour, 0, 0)
        .single()
        .expect("a real instant")
}

fn note(n: u128, question: Option<u128>, answer: Option<u128>) -> NoteRecord {
    NoteRecord {
        id: Uuid::from_u128(n),
        question_id: question.map(Uuid::from_u128),
        answer_id: answer.map(Uuid::from_u128),
        author: "Chuck".to_string(),
        text: format!("note {n}"),
        created_at: at(17, 16),
        struck_at: None,
        struck_by: None,
    }
}

/// A standing note carries its day and no struck line.
#[test]
fn a_standing_note_has_its_day_and_no_struck_line() {
    let dto = note_dto(&settings(), &note(1, Some(10), None));
    assert_eq!(dto.when, "17 Sep");
    assert_eq!(dto.struck, None);
    assert_eq!(
        (dto.author.as_str(), dto.text.as_str()),
        ("Chuck", "note 1")
    );
}

/// A struck note says so, with the day it was struck — and no raw placeholder.
#[test]
fn a_struck_note_says_struck_with_its_own_day() {
    let mut withdrawn = note(2, Some(10), None);
    withdrawn.struck_at = Some(at(18, 15));
    withdrawn.struck_by = Some("chuck".to_string());
    let dto = note_dto(&settings(), &withdrawn);
    assert_eq!(dto.struck.as_deref(), Some("struck 18 Sep"));
}

/// A deck row shows notes on the question and on its CURRENT answer only.
#[test]
fn a_row_shows_question_notes_and_current_answer_notes_only() {
    let notes = vec![
        note(1, Some(10), None),      // on the question
        note(2, Some(10), Some(100)), // on the current answer
        note(3, Some(10), Some(99)),  // on a superseded attempt
        note(4, Some(11), None),      // another question
        note(5, None, None),          // the scenario
    ];
    let shown = row_notes(
        &settings(),
        &notes,
        Uuid::from_u128(10),
        Some(Uuid::from_u128(100)),
    );
    let ids: Vec<Uuid> = shown.iter().map(|n| n.id).collect();
    assert_eq!(ids, vec![Uuid::from_u128(1), Uuid::from_u128(2)]);
}

/// With no standing answer, no attempt-level note can be the current one.
#[test]
fn an_unanswered_row_shows_only_question_level_notes() {
    let notes = vec![note(1, Some(10), None), note(2, Some(10), Some(100))];
    let shown = row_notes(&settings(), &notes, Uuid::from_u128(10), None);
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].id, Uuid::from_u128(1));
}
