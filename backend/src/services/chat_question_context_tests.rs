//! Tests for the rendered context block.

use chrono::{TimeZone, Utc};
use uuid::Uuid;

use super::*;
use crate::services::chat_question_text::has_internal_key;

fn attempt(answer: &str, read: Option<&str>, ok: Option<bool>) -> AttemptRecord {
    AttemptRecord {
        id: Uuid::nil(),
        question_text: "q".into(),
        answer_text: answer.into(),
        answered_at: Utc.with_ymd_and_hms(2026, 9, 19, 12, 0, 0).unwrap(),
        mark: "answered".into(),
        read_text: read.map(str::to_string),
        read_ok: ok,
        read_error: None,
        read_abstain_reason: None,
        self_check: serde_json::json!({}),
        help_opened: false,
        points_to: None,
    }
}

fn context() -> QuestionContext {
    QuestionContext {
        scenario_code: "S-13".into(),
        question_text: "You waited thirteen years?".into(),
        kind: "cross".into(),
        asker: "the defense".into(),
        tactic_name: Some("Bait the delay".into()),
        attack: Some("She sat on it.".into()),
        watch_for: None,
        points: vec![(
            1,
            "They never told me.".into(),
            Some("the 2016 response".into()),
        )],
        pair_said: Some("\"we told her\"".into()),
        pair_admitted: None,
        attempts: vec![attempt(
            "I never knew until December 2025",
            Some("Fine. P1 holds; S2 sits unanswered and R1 backs it."),
            Some(true),
        )],
        notes: vec![],
        siblings: vec![SiblingThread {
            owner_name: "Chuck".into(),
            lines: vec![("Chuck".into(), "Cleanest exhibit?".into())],
        }],
        earlier: vec![DiscussionTurnRecord {
            id: Uuid::nil(),
            question_id: Uuid::nil(),
            author_user_id: "roman".into(),
            author_name: "Roman".into(),
            role: "user".into(),
            model_id: None,
            text: "Old team note about S1.".into(),
            input_tokens: None,
            output_tokens: None,
            ms: None,
            created_at: Utc.with_ymd_and_hms(2026, 9, 17, 12, 0, 0).unwrap(),
        }],
        witness_name: "Marie".into(),
        viewer_name: "Marie".into(),
        case_timezone: "America/Detroit".into(),
    }
}

/// Roman's fatal-flaw ruling: no internal key reaches the model — not from a
/// stored read, not from the old dock's history.
#[test]
fn the_package_never_contains_internal_keys() {
    let text = render_context(&context());
    assert!(!has_internal_key(&text), "a key survived:\n{text}");
    assert!(text.contains("talking point 1 holds"));
    assert!(text.contains("what they admitted under oath sits unanswered"));
    assert!(text.contains("Old team note about what they said."));
}

#[test]
fn every_section_is_present_and_an_empty_one_says_so() {
    let text = render_context(&context());
    for heading in [
        "## THE QUESTION",
        "## THE ATTACK THIS SCENARIO ANSWERS",
        "## MARIE'S TALKING POINTS",
        "## THE SWORN PAIR",
        "## MARIE'S ANSWERS AND THEIR READS, OLDEST FIRST",
        "## STANDING NOTES FROM COUNSEL AND THE TEAM\n(none recorded)",
        "## THE OTHER THREADS ON THIS QUESTION\n### Chuck's thread\nChuck: Cleanest exhibit?",
        "## THE EARLIER SHARED DISCUSSION OF THIS QUESTION\nRoman (",
    ] {
        assert!(text.contains(heading), "missing {heading:?} in:\n{text}");
    }
    assert!(text.contains("Read (marked fine):"));
    assert!(text.contains("What they admitted under oath: (none recorded)"));
}

/// ADDENDUM_1: a FAILED analysis reaches the model as "no answer analysis (it
/// failed)" — never as Marie's failure sentence.
#[test]
fn a_failed_analysis_is_named_as_failed_not_passed_on_as_her_sentence() {
    let mut failed = attempt(
        "My answer.",
        Some("Your answer is saved, but the answer analysis didn't come back this time."),
        None,
    );
    failed.read_error = Some("the call failed: timeout".into());
    failed.read_abstain_reason = Some("the model could not be reached".into());
    let mut c = context();
    c.attempts = vec![failed];

    let text = render_attempts(&c);
    assert!(
        text.contains("Read: no answer analysis (it failed)"),
        "{text}"
    );
    assert!(!text.contains("didn't come back"), "{text}");
}

/// A MODEL decline is not a failure: its sentence still reaches the model.
#[test]
fn a_model_decline_still_passes_its_sentence_to_the_model() {
    let mut declined = attempt(
        "test",
        Some("The analysis couldn't judge this answer. That looks like a test entry."),
        None,
    );
    declined.read_error = Some("the model abstained: That looks like a test entry.".into());
    declined.read_abstain_reason = Some("That looks like a test entry.".into());
    let mut c = context();
    c.attempts = vec![declined];

    let text = render_attempts(&c);
    assert!(
        text.contains(
            "Read: The analysis couldn't judge this answer. That looks like a test entry."
        ),
        "{text}"
    );
}
