// Tests for `services::practice_notes`.
//
// Small module, two things worth pinning. The struck line, because its PRESENCE
// is what strikes the text through on screen — a note rendered struck with no
// statement of when invites the reader to assume it never really was. And the
// author fence, because it is the whole of who may sign a note about a witness's
// testimony, and it is a stored vocabulary rather than a compiled one.

use super::*;
use crate::domain::settings::Settings;
use chrono::TimeZone;
use uuid::Uuid;

fn at(day: u32) -> chrono::DateTime<chrono::Utc> {
    chrono::Utc
        .with_ymd_and_hms(2026, 8, day, 9, 0, 0)
        .single()
        .expect("a real instant")
}

fn note(
    n: u128,
    question: Option<u128>,
    answer: Option<u128>,
    author: &str,
    day: u32,
) -> NoteRecord {
    NoteRecord {
        id: Uuid::from_u128(n),
        question_id: question.map(Uuid::from_u128),
        answer_id: answer.map(Uuid::from_u128),
        author: author.to_string(),
        text: format!("note {n}"),
        created_at: at(day),
        struck_at: None,
        struck_by: None,
    }
}

/// A standing note carries no struck line.
#[test]
fn a_standing_note_carries_no_struck_line() {
    let dto = note_dto(&Settings::for_test(), &note(1, None, None, "Chuck", 18));
    assert_eq!(dto.when, "Tue 18 Aug");
    assert!(dto.struck.is_none());
}

/// A struck note says WHEN it was struck.
#[test]
fn a_struck_note_says_when_it_was_struck() {
    let mut record = note(1, None, None, "Roman", 17);
    record.struck_at = Some(at(18));
    record.struck_by = Some("Roman".to_string());

    let dto = note_dto(&Settings::for_test(), &record);
    assert_eq!(dto.struck.as_deref(), Some("struck Tue 18 Aug"));
    assert_eq!(dto.text, "note 1", "the note itself is untouched");
}

/// The scenario panel shows only notes that name no question.
#[test]
fn the_scenario_panel_takes_only_the_notes_about_the_scenario() {
    let s = Settings::for_test();
    let notes = vec![
        note(1, None, None, "Chuck", 18),
        note(2, Some(9), None, "Chuck", 18),
        note(3, Some(9), Some(7), "Marie", 18),
    ];
    let shown = scenario_notes(&s, &notes);
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].id, Uuid::from_u128(1));
}

/// The count is of UNSTRUCK notes newer than her last sitting.
#[test]
fn only_unstruck_notes_newer_than_her_last_sitting_are_counted() {
    let mut struck = note(3, None, None, "Chuck", 19);
    struck.struck_at = Some(at(19));
    struck.struck_by = Some("Chuck".to_string());

    let notes = vec![
        note(1, None, None, "Roman", 17),
        note(2, None, None, "Chuck", 19),
        struck,
    ];
    let (count, who) = new_since(&notes, Some(at(18)));
    assert_eq!(count, 1, "the old one and the struck one do not count");
    assert_eq!(who, Some("Chuck"));
}

/// With no sitting behind her, every standing note is new.
#[test]
fn with_no_previous_sitting_every_standing_note_counts() {
    let notes = vec![
        note(1, None, None, "Roman", 17),
        note(2, None, None, "Chuck", 18),
    ];
    let (count, who) = new_since(&notes, None);
    assert_eq!(count, 2);
    // The list arrives oldest first, so the NEWEST author is the last of them.
    assert_eq!(who, Some("Chuck"));
}

/// The author fence reads the STORED vocabulary.
#[test]
fn the_author_fence_reads_the_stored_vocabulary() {
    let s = Settings::for_test();
    for name in ["Chuck", "Marie", "Roman"] {
        assert!(
            is_note_author(&s, name),
            "{name} should be able to write a note"
        );
    }
    assert!(
        !is_note_author(&s, "George"),
        "opposing counsel does not write notes here"
    );
    assert!(!is_note_author(&s, ""), "an unsigned note is refused");
}

/// The comparison is EXACT, not case-folded.
///
/// The name is stored on the note and printed beside it, so accepting "chuck"
/// would put two spellings of one person on one panel.
#[test]
fn the_author_fence_is_exact_and_not_case_folded() {
    assert!(!is_note_author(&Settings::for_test(), "chuck"));
}

/// Only the editors may edit, and the list is SHORTER than the note authors.
///
/// Marie answers the deck; she does not edit it. That ruling is Roman's to
/// change without a build, which is why both lists are stored rows.
#[test]
fn marie_may_write_a_note_and_may_not_edit_the_deck() {
    let s = Settings::for_test();
    assert!(is_note_author(&s, "Marie"));
    assert!(!is_editor(&s, "Marie"));
    assert!(is_editor(&s, "Chuck") && is_editor(&s, "Roman"));
}

/// The refusal messages list the stored names, so a 400 says what to type.
#[test]
fn the_refusal_messages_name_the_stored_lists() {
    let s = Settings::for_test();
    assert_eq!(author_list(&s), "Chuck, Marie, Roman");
    assert_eq!(editor_list(&s), "Chuck, Roman");
}
