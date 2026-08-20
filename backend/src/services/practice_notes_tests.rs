// Tests for `services::practice_notes`.
//
// Small module, two things worth pinning. The struck line, because its PRESENCE
// is what strikes the text through on screen — a note rendered struck with no
// statement of when invites the reader to assume it never really was. And the
// author fence, because it is the whole of who may sign a note about a witness's
// testimony, and it is a stored vocabulary rather than a compiled one.

use super::*;
use crate::auth::AuthUser;
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

// ── Attribution comes from the login (hotfix, 2026-08-19) ────────────────────
//
// The four tests that stood here checked a stored allow-list of display names
// against a selector's value. Both are gone: the selector was the fault Roman
// hit in the first minute of .402, and the allow-list behind it could only ever
// lock a real signed-in user out. What replaces them is one function, and what
// it must not do is invent a name.

fn user(username: &str, display: &str) -> AuthUser {
    AuthUser {
        username: username.to_string(),
        email: String::new(),
        display_name: display.to_string(),
        groups: vec![],
    }
}

/// The id is the username; the printed name is the display name.
#[test]
fn attribution_takes_the_id_and_the_name_from_the_login() {
    let (id, name) = attribution(&user("chuck", "Chuck"));
    assert_eq!(id, "chuck");
    assert_eq!(name, "Chuck");
}

/// A blank display name falls back to the username, never to an empty author.
///
/// Authentik can hold an account with no name set. A note whose author renders
/// as nothing is a note nobody can answer — and this is the one place a name
/// could have arrived empty, because everything else on these tables is either
/// a stored row or something a human typed.
#[test]
fn a_blank_display_name_falls_back_to_the_username() {
    let (id, name) = attribution(&user("marie", "   "));
    assert_eq!(id, "marie");
    assert_eq!(name, "marie", "never an empty author");
}

/// Nothing here consults a stored list, and nothing can refuse a real user.
///
/// ANTI-REGRESSION: the two settings rows this replaced
/// (`practice_note_authors`, `practice_editor_authors`) are deleted by the
/// hotfix migration. If somebody reintroduces a vocabulary check, a signed-in
/// user whose Authentik display name is spelled differently from the row would
/// be silently unable to write a note — which is the class of fault this whole
/// task exists to remove.
#[test]
fn any_signed_in_user_is_attributable() {
    for (u, d) in [
        ("roman", "Roman"),
        ("chuck", "Chuck"),
        ("marie", "Marie"),
        ("j.doe", "J. Doe"),
    ] {
        let (id, name) = attribution(&user(u, d));
        assert_eq!(id, u);
        assert_eq!(name, d);
    }
}
