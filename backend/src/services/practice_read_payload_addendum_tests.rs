// Tests for the three sections CC_TASK_QUESTION_CHAT_ADDENDUM_v1 added to the
// read's package: the attack, the parent question (redirects only), the notes.
//
// Split from `practice_read_payload_tests.rs` under Rule 17; the whole-body pin
// of the full message stays there, where it always was.

use super::tests::cross_payload;
use super::*;

/// All three new sections reach the model, in their places.
#[test]
fn the_attack_the_parent_and_the_notes_all_reach_the_model() {
    let message = build_user_message(&ReadPayload {
        kind: "redirect".to_string(),
        parent: Parent::Repairs("You never showed up to the meeting, did you?".to_string()),
        ..cross_payload()
    });
    assert!(message.starts_with(
        "THE ATTACK THIS SCENARIO ANSWERS:\nMarie refused to divide the property.\n\n"
    ));
    assert!(message.contains(
        "THE TACTIC: compound\n\nTHE QUESTION THIS REDIRECT REPAIRS:\nYou never showed up to the meeting, did you?\n\nHER ANSWER, verbatim:"
    ));
    assert!(message.contains(
        "- Chuck, 17 Sep: Lead with the letter.\n- Roman, 15 Sep: Keep it under ten words."
    ));
}

/// (M) A non-redirect never carries a parent section — not even an empty one.
#[test]
fn a_non_redirect_never_carries_a_parent_section() {
    for kind in ["cross", "direct"] {
        let message = build_user_message(&ReadPayload {
            kind: kind.to_string(),
            parent: Parent::NotARedirect,
            ..cross_payload()
        });
        assert!(
            !message.contains("REDIRECT REPAIRS"),
            "{kind} carried a parent section"
        );
    }
}

/// The named absences render when the three are empty — never a silent gap.
#[test]
fn the_three_named_absences_render_when_empty() {
    let message = build_user_message(&ReadPayload {
        kind: "redirect".to_string(),
        attack: None,
        parent: Parent::Unresolved,
        notes: Vec::new(),
        ..cross_payload()
    });
    assert!(message.contains(&format!("THE ATTACK THIS SCENARIO ANSWERS:\n{NO_ATTACK}\n")));
    assert!(message.contains(&format!(
        "THE QUESTION THIS REDIRECT REPAIRS:\n{NO_PARENT}\n"
    )));
    assert!(message.contains(&format!("newest first:\n{NO_NOTES}\n")));
}

/// The new sections add no citable key: a note or the attack is guidance, not a
/// receipt, and the model may not cite it as one.
#[test]
fn the_new_sections_add_no_citable_keys() {
    assert_eq!(
        cross_payload()
            .citable_keys()
            .into_iter()
            .collect::<Vec<_>>()
            .join(" "),
        "P1 P2 P3 R1 R2 R3 S1 S2"
    );
}
