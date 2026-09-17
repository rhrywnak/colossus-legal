// Tests for `services::practice_read_context` — what the read now sees.
//
// CC_TASK_QUESTION_CHAT_ADDENDUM_v1. The two decisions that shape the model's
// input without a database: which parent a redirect shows, and which notes stand.

use chrono::{DateTime, TimeZone, Utc};
use uuid::Uuid;

use super::*;

fn at(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, day, 16, 0, 0)
        .single()
        .expect("a real instant")
}

fn question(
    kind: &str,
    deck_key: Option<&str>,
    follows: Option<&str>,
    text: &str,
) -> PracticeQuestionRecord {
    PracticeQuestionRecord {
        id: Uuid::new_v4(),
        scenario_id: Uuid::nil(),
        side: if kind == "cross" { "george" } else { "chuck" }.to_string(),
        text: text.to_string(),
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
        sort_order: 1,
        flag_note: None,
        deck_key: deck_key.map(str::to_string),
        kind: kind.to_string(),
        follows_key: follows.map(str::to_string),
        source_line: None,
        hidden_at: None,
        draft_by: None,
        updated_at: at(1),
    }
}

fn note(author: &str, day: u32, answer: Option<Uuid>, struck: bool) -> NoteRecord {
    NoteRecord {
        id: Uuid::new_v4(),
        question_id: Some(Uuid::nil()),
        answer_id: answer,
        author: author.to_string(),
        text: format!("{author} on day {day}"),
        created_at: at(day),
        struck_at: struck.then(|| at(day + 1)),
        struck_by: struck.then(|| "chuck".to_string()),
    }
}

// ─── the parent ──────────────────────────────────────────────────────────────

/// A redirect shows the defense question it repairs.
#[test]
fn a_redirect_repairs_the_question_its_follows_key_names() {
    let deck = vec![
        question("cross", Some("g1"), None, "You never showed up, did you?"),
        question("cross", Some("g2"), None, "Another question"),
    ];
    let redirect = question("redirect", Some("r1"), Some("g1"), "Tell the jury why.");
    assert_eq!(
        parent_from(&redirect, &deck),
        Parent::Repairs("You never showed up, did you?".to_string())
    );
}

/// (M) A non-redirect never has a parent — even one whose follows_key resolves.
#[test]
fn a_non_redirect_is_never_given_a_parent() {
    let deck = vec![question(
        "cross",
        Some("g1"),
        None,
        "You never showed up, did you?",
    )];
    for kind in ["cross", "direct"] {
        let q = question(kind, Some("x1"), Some("g1"), "Q");
        assert_eq!(parent_from(&q, &deck), Parent::NotARedirect, "{kind}");
    }
}

/// A hand-added redirect with no resolvable parent sends the named absence.
#[test]
fn an_unresolvable_redirect_is_unresolved_not_silent() {
    let deck = vec![question("cross", Some("g1"), None, "Q")];
    assert_eq!(
        parent_from(&question("redirect", None, None, "R"), &deck),
        Parent::Unresolved
    );
    assert_eq!(
        parent_from(&question("redirect", None, Some("g9"), "R"), &deck),
        Parent::Unresolved
    );
}

// ─── the notes ───────────────────────────────────────────────────────────────

/// (M) A struck note never reaches the model.
#[test]
fn a_struck_note_is_never_sent() {
    let notes = vec![
        note("Chuck", 10, None, true),
        note("Roman", 11, None, false),
    ];
    let sent = standing_notes(&notes, None, "America/Detroit");
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].author, "Roman");
    assert!(sent.iter().all(|n| !n.text.contains("Chuck")));
}

/// Every author is sent, labelled and dated, newest first; a note on a
/// superseded attempt is not.
#[test]
fn standing_notes_are_every_authors_newest_first_on_the_current_answer() {
    let current = Uuid::new_v4();
    let old_attempt = Uuid::new_v4();
    let notes = vec![
        note("Roman", 12, None, false),
        note("Chuck", 15, Some(current), false),
        note("Chuck", 14, Some(old_attempt), false),
    ];
    let sent = standing_notes(&notes, Some(current), "America/Detroit");
    assert_eq!(
        sent.iter()
            .map(|n| (n.author.as_str(), n.when.as_str()))
            .collect::<Vec<_>>(),
        vec![("Chuck", "15 Sep"), ("Roman", "12 Sep")]
    );
}

/// No notes is an empty list — the payload then sends the named absence.
#[test]
fn no_notes_is_empty() {
    assert!(standing_notes(&[], None, "America/Detroit").is_empty());
}
