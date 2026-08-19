// Tests for `services::practice_changes`.
//
// Two readers, one vocabulary, and one arithmetic mistake that would matter: a
// `changed` badge that never retires, or a box that lights up when nothing has
// changed. Neither is visible in a screenshot and neither is a type error — the
// strings are well-formed whatever they say.

use super::*;
use crate::domain::settings::Settings;
use crate::repositories::pipeline_repository::practice::PracticeQuestionRecord;
use chrono::TimeZone;

fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, day, hour, 0, 0)
        .single()
        .expect("a real instant")
}

fn question(n: u128) -> PracticeQuestionRecord {
    PracticeQuestionRecord {
        id: Uuid::from_u128(n),
        scenario_id: Uuid::nil(),
        side: "george".to_string(),
        text: format!("question {n}"),
        tactic: None,
        braid_rows: None,
        source_kind: "manual".to_string(),
        source_ref: None,
        receipt: None,
        watch_for: None,
        stronger: None,
        stronger_lean: None,
        pair_said: None,
        pair_admitted: None,
        sort_order: i32::try_from(n).unwrap_or(1),
        flag_note: None,
        deck_key: None,
        kind: "cross".to_string(),
        follows_key: None,
        source_line: None,
        hidden_at: None,
        draft_by: None,
    }
}

fn change(n: u128, kind: &str, by: &str, when: DateTime<Utc>) -> DeckChangeRecord {
    DeckChangeRecord {
        question_id: Uuid::from_u128(n),
        change_kind: kind.to_string(),
        field: None,
        after_value: None,
        changed_by: by.to_string(),
        changed_at: when,
    }
}

/// Nothing changed and no notes arrived: NO box.
///
/// A box reading "0 questions changed" is a screen telling a witness to re-read
/// a deck that is exactly as she left it.
#[test]
fn nothing_changed_withdraws_the_box_entirely() {
    let s = Settings::for_test();
    assert!(changed_box(&s, &[question(1)], &[], 0, None).is_none());
}

/// The heading names the count, the NEWEST editor, and the newest day.
#[test]
fn the_heading_names_the_count_the_newest_editor_and_the_day() {
    let s = Settings::for_test();
    let deck = vec![question(1), question(2), question(3)];
    // Newest first, which is the order the repository returns.
    let changes = vec![
        change(3, "moved", "Chuck", at(19, 9)),
        change(1, "reworded", "Roman", at(18, 9)),
    ];

    let box_ = changed_box(&s, &deck, &changes, 0, None).expect("something changed");
    assert_eq!(
        box_.heading,
        "Changed since your last sitting: 2 questions — Chuck, Wed 19 Aug"
    );
    assert!(!box_.heading.contains('{'), "a placeholder survived");
}

/// A sitting where ONLY notes arrived still gets the box.
///
/// "2 new notes — Chuck" is exactly the thing she opened the screen to be told,
/// and requiring a deck change first would hide it.
#[test]
fn notes_alone_are_enough_to_raise_the_box() {
    let s = Settings::for_test();
    let box_ = changed_box(&s, &[question(1)], &[], 2, Some("Chuck")).expect("notes arrived");
    assert!(
        box_.heading.contains("2 new notes — Chuck"),
        "{}",
        box_.heading
    );
    assert!(box_.items.is_empty(), "no deck change, so nothing to list");
}

/// Each change becomes a sentence naming the question's PRINTED position.
#[test]
fn every_change_kind_becomes_its_own_sentence() {
    let s = Settings::for_test();
    let deck = vec![question(1), question(2)];
    let changes = vec![
        change(1, "reworded", "Chuck", at(19, 9)),
        change(2, "moved", "Chuck", at(19, 8)),
        change(2, "hidden", "Chuck", at(19, 7)),
        change(1, "unhidden", "Chuck", at(19, 6)),
    ];

    let box_ = changed_box(&s, &deck, &changes, 0, None).expect("changes");
    assert_eq!(
        box_.items,
        vec![
            "Q1 re-worded".to_string(),
            "Q2 moved".to_string(),
            "Q2 hidden".to_string(),
            "Q1 put back".to_string(),
        ]
    );
}

/// An `added` change names the side it was added on.
#[test]
fn an_added_question_names_its_side() {
    let s = Settings::for_test();
    let mut added = change(2, "added", "Chuck", at(19, 9));
    added.after_value = Some("chuck".to_string());
    let box_ = changed_box(&s, &[question(1), question(2)], &[added], 0, None).expect("a change");
    assert_eq!(box_.items, vec!["new: Q2 (chuck)".to_string()]);
}

/// An `edited` change names the field.
#[test]
fn an_edited_question_names_the_field_that_changed() {
    let s = Settings::for_test();
    let mut edited = change(1, "edited", "Roman", at(19, 9));
    edited.field = Some("watch_for".to_string());
    let box_ = changed_box(&s, &[question(1)], &[edited], 0, None).expect("a change");
    assert_eq!(box_.items, vec!["Q1 — watch_for changed".to_string()]);
}

/// An unknown change kind is SHOWN, never dropped.
///
/// The column's CHECK permits six values, so this is unreachable through the
/// database — which is the point: a seventh added to a migration and not to the
/// match must appear in the list rather than vanish from the record of what
/// changed.
#[test]
fn an_unknown_change_kind_renders_itself_rather_than_disappearing() {
    let s = Settings::for_test();
    let box_ = changed_box(
        &s,
        &[question(1)],
        &[change(1, "superseded", "Chuck", at(19, 9))],
        0,
        None,
    )
    .expect("a change");
    assert_eq!(box_.items, vec!["superseded".to_string()]);
}

/// The badge stands on a question she has NOT answered since it changed.
#[test]
fn a_changed_question_she_has_not_answered_since_wears_the_badge() {
    let changed = change(1, "reworded", "Chuck", at(19, 9));
    assert_eq!(badged(&[changed], &[]), vec![Uuid::from_u128(1)]);
}

/// Answering it after the change RETIRES the badge.
///
/// The badge asks her to re-read the question; answering it is doing that, and a
/// badge nothing retires is a badge she stops seeing.
#[test]
fn answering_after_the_change_retires_the_badge() {
    let changed = change(1, "reworded", "Chuck", at(19, 9));
    let answered = vec![(Uuid::from_u128(1), at(19, 10))];
    assert!(badged(&[changed], &answered).is_empty());
}

/// Answering BEFORE the change does not retire it.
///
/// The off-by-one that matters: comparing the wrong way round would clear the
/// badge on every question she has ever answered, which is most of them.
#[test]
fn answering_before_the_change_leaves_the_badge_standing() {
    let changed = change(1, "reworded", "Chuck", at(19, 9));
    let answered = vec![(Uuid::from_u128(1), at(18, 10))];
    assert_eq!(badged(&[changed], &answered), vec![Uuid::from_u128(1)]);
}

/// Two changes to one question badge it ONCE.
#[test]
fn two_changes_to_one_question_badge_it_once() {
    let changes = vec![
        change(1, "reworded", "Chuck", at(19, 9)),
        change(1, "moved", "Chuck", at(19, 8)),
    ];
    assert_eq!(badged(&changes, &[]), vec![Uuid::from_u128(1)]);
}

/// Chuck's sheet prints the same sentence, with the editor's name after it.
#[test]
fn the_sheet_prints_the_change_and_who_made_it() {
    let s = Settings::for_test();
    let lines = sheet_lines(
        &s,
        &[question(1)],
        &[change(1, "reworded", "Chuck", at(19, 9))],
    );
    assert_eq!(lines, vec!["Q1 re-worded — Chuck".to_string()]);
}
