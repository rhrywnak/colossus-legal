// Tests for `services::practice_editor_options`.
//
// The add form's picker. What it can get wrong is silent: an option whose index
// is off by one attaches a new question to the wrong ruled instance, and the
// screen then shows a receipt about a different page of the record under a
// question nobody meant to bind there.

use super::*;
use crate::domain::settings::Settings;
use uuid::Uuid;

fn instance(n: u128, source_line: Option<&str>) -> PracticeQuestionRecord {
    PracticeQuestionRecord {
        id: Uuid::from_u128(n),
        scenario_id: Uuid::nil(),
        side: "george".to_string(),
        text: format!("question {n}"),
        tactic: Some(4),
        braid_rows: None,
        source_kind: "instance".to_string(),
        source_ref: Some(format!("doc:evidence:{n}")),
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
        source_line: source_line.map(str::to_string),
        hidden_at: None,
        draft_by: None,
    }
}

fn chuck(n: u128) -> PracticeQuestionRecord {
    let mut q = instance(n, None);
    q.side = "chuck".to_string();
    q.kind = "direct".to_string();
    q.source_kind = "point".to_string();
    q.tactic = None;
    q
}

fn point(position: i32, text: &str) -> PracticePointRecord {
    PracticePointRecord {
        position,
        text: text.to_string(),
        exhibit: None,
    }
}

/// The instances come first, labelled from each question's own source line.
#[test]
fn the_picker_labels_each_instance_from_its_questions_source_line() {
    let s = Settings::for_test();
    let deck = vec![
        instance(1, Some("Hearing, 15 Dec 2009, p. 34")),
        instance(2, Some("Hearing, 15 Dec 2009, p. 33")),
    ];

    let options = attach_options(&s, &deck, &[]);

    assert_eq!(options.len(), 2);
    assert_eq!(options[0].source_kind, "instance");
    assert_eq!(options[0].source_index, 1);
    assert_eq!(options[0].label, "instance 1 — Hearing, 15 Dec 2009, p. 34");
    assert_eq!(options[1].source_index, 2);
    assert_eq!(options[1].label, "instance 2 — Hearing, 15 Dec 2009, p. 33");
}

/// The points follow, labelled with her own words.
#[test]
fn the_points_follow_the_instances_and_carry_their_own_text() {
    let s = Settings::for_test();
    let options = attach_options(
        &s,
        &[instance(1, Some("the hearing, p. 34"))],
        &[
            point(1, "I asked in writing"),
            point(2, "they got my letter"),
        ],
    );

    assert_eq!(options.len(), 3);
    assert_eq!(options[1].source_kind, "point");
    assert_eq!(options[1].label, "point 1 — I asked in writing");
    assert_eq!(options[2].label, "point 2 — they got my letter");
}

/// An instance whose question carries no source line still gets an option.
///
/// The template's `{text}` renders empty rather than the machine cutting a
/// phrase out of the receipt paragraph — the same choice, and the same reason,
/// as the "I'd point to…" list. Losing the option entirely would make an
/// instance un-attachable because nobody wrote a handle for it.
#[test]
fn an_instance_with_no_source_line_is_still_offered() {
    let s = Settings::for_test();
    let options = attach_options(&s, &[instance(1, None)], &[]);
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].source_index, 1);
    assert!(
        options[0].label.starts_with("instance 1 —"),
        "{}",
        options[0].label
    );
}

/// Only INSTANCE questions count toward the instance list.
///
/// Chuck's questions bind to points. Counting them would offer "instance 6" on a
/// scenario with five, and attaching to it would fail at the fence — a control
/// that cannot do what it says.
#[test]
fn chuck_questions_do_not_inflate_the_instance_count() {
    let s = Settings::for_test();
    let deck = vec![
        instance(1, Some("p. 34")),
        chuck(2),
        chuck(3),
        instance(4, Some("p. 33")),
    ];
    let instances: Vec<_> = attach_options(&s, &deck, &[])
        .into_iter()
        .filter(|o| o.source_kind == "instance")
        .collect();
    assert_eq!(instances.len(), 2, "two instance questions, two options");
}

/// A scenario with neither offers an EMPTY list.
///
/// Not a failure: the add form still has "no receipt", which is its own stored
/// option and the honest answer for a question that traces to nothing.
#[test]
fn a_scenario_with_nothing_to_attach_to_offers_nothing() {
    assert!(attach_options(&Settings::for_test(), &[], &[]).is_empty());
}

/// No option ships a raw placeholder.
#[test]
fn no_label_ships_a_raw_placeholder() {
    let s = Settings::for_test();
    for option in attach_options(&s, &[instance(1, None)], &[point(1, "a point")]) {
        assert!(
            !option.label.contains('{'),
            "a placeholder survived: {}",
            option.label
        );
    }
}
