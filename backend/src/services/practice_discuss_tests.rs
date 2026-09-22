// Tests for `services::practice_discuss` — what the dock's model is told.

use chrono::{TimeZone, Utc};
use uuid::Uuid;

use super::*;
use crate::services::practice_read_payload::{
    build_user_message, Keyed, Parent, PayloadNote, PointsTo, ReadPayload, Tactic,
};

fn payload(answer: &str) -> ReadPayload {
    ReadPayload {
        question: "You never showed up to the meeting, did you?".to_string(),
        side: "the defense".to_string(),
        kind: "cross".to_string(),
        tactic: Tactic::Named("half-truth".to_string()),
        answer: answer.to_string(),
        points: vec![Keyed::new("P1", Some("I asked in writing.".to_string()))],
        receipts: vec![Keyed::new(
            "R1",
            Some("certified letter, 16 Nov 2009".to_string()),
        )],
        said: None,
        admitted: None,
        points_to: PointsTo::NeverOpened,
        watch_for: None,
        always: "Tell the truth".to_string(),
        attack: Some("Marie refused to divide the property.".to_string()),
        parent: Parent::NotARedirect,
        notes: vec![PayloadNote {
            author: "Chuck".to_string(),
            when: "17 Sep".to_string(),
            text: "Lead with the letter.".to_string(),
        }],
    }
}

fn turn(author: &str, role: &str, text: &str, minute: u32) -> DiscussionTurnRecord {
    DiscussionTurnRecord {
        id: Uuid::new_v4(),
        question_id: Uuid::nil(),
        author_user_id: "docmarie".to_string(),
        author_name: author.to_string(),
        role: role.to_string(),
        model_id: (role == "model").then(|| "claude-opus-5".to_string()),
        text: text.to_string(),
        input_tokens: None,
        output_tokens: None,
        ms: None,
        created_at: Utc
            .with_ymd_and_hms(2026, 9, 17, 18, minute, 0)
            .single()
            .expect("instant"),
    }
}

/// The composed input carries her answer, the P/R keys, the analysis, the notes,
/// the prior turns in order, and the new message.
#[test]
fn the_input_carries_the_package_the_analysis_and_the_thread() {
    let package = build_user_message(&payload("My certified letter asked for exactly that."));
    let thread = vec![
        turn("Marie", "user", "How do I say this?", 52),
        turn("Claude Opus 5", "model", "Lead with the letter.", 53),
    ];
    let input = compose_discussion_input(
        &package,
        Some("Fine. Short, and yours."),
        false,
        &thread,
        "Marie",
        "What if he pushes?",
    );
    assert!(input.starts_with(&package));
    for needle in [
        "HER ANSWER, verbatim:\nMy certified letter asked for exactly that.",
        "P1. I asked in writing.",
        "R1. certified letter, 16 Nov 2009",
        "- Chuck, 17 Sep: Lead with the letter.",
        "THE LATEST ANALYSIS OF HER SAVED ANSWER:\nFine. Short, and yours.",
        "THE CONVERSATION SO FAR, oldest first:\nMarie: How do I say this?\n\nClaude Opus 5: Lead with the letter.",
        "THE NEW MESSAGE, from Marie:\nWhat if he pushes?",
    ] {
        assert!(input.contains(needle), "missing: {needle}");
    }
    assert!(
        !input.contains("UNSAVED DRAFT"),
        "a saved answer is not a draft"
    );
}

/// (M) A draft is flagged to the model as unsaved.
#[test]
fn a_draft_is_named_as_an_unsaved_draft() {
    let package = build_user_message(&payload("a half-written draft"));
    let input = compose_discussion_input(&package, None, true, &[], "Marie", "Is this better?");
    assert!(input.contains("HER ANSWER, verbatim:\na half-written draft"));
    assert!(input.contains("UNSAVED DRAFT"));
}

/// The absences are said, not omitted.
#[test]
fn a_first_message_with_no_analysis_says_so() {
    let input = compose_discussion_input("PACKAGE", None, false, &[], "Chuck", "Thoughts?");
    assert!(input.contains(NO_ANALYSIS));
    assert!(input.contains(NO_CONVERSATION));
}

/// (M) The cap refuses at the limit, not one past it.
#[test]
fn the_cap_refuses_at_the_limit() {
    assert!(!cap_reached(39, 40));
    assert!(cap_reached(40, 40));
    assert!(cap_reached(41, 40));
}

/// The dock's turns carry the composed clock time and the author.
#[test]
fn thread_dtos_compose_author_and_time() {
    let dtos = thread_dtos(
        &[turn("Claude Opus 5", "model", "Lead.", 52)],
        "America/Detroit",
    );
    assert_eq!(dtos[0].author, "Claude Opus 5");
    assert_eq!(dtos[0].model_id.as_deref(), Some("claude-opus-5"));
    assert!(!dtos[0].when.is_empty() && !dtos[0].when.contains('{'));
}

/// (M) The default model is the settings row's, not a literal: change the row and
/// the choice follows. An explicit request wins.
#[test]
fn the_default_model_follows_the_settings_row() {
    let known = |id: &str| ["claude-opus-5", "claude-sonnet-5"].contains(&id);
    assert_eq!(
        choose_model(None, "claude-opus-5", known),
        Ok("claude-opus-5".to_string())
    );
    assert_eq!(
        choose_model(None, "claude-sonnet-5", known),
        Ok("claude-sonnet-5".to_string())
    );
    assert_eq!(
        choose_model(Some("claude-sonnet-5"), "claude-opus-5", known),
        Ok("claude-sonnet-5".to_string())
    );
}

/// An unknown model is refused by id — the route's 400.
#[test]
fn an_unknown_model_is_refused_by_id() {
    let known = |id: &str| id == "claude-opus-5";
    assert_eq!(
        choose_model(Some("gpt-9"), "claude-opus-5", known),
        Err("gpt-9".to_string())
    );
    assert_eq!(
        choose_model(None, "retired-model", known),
        Err("retired-model".to_string())
    );
}

fn saved(text: &str) -> StandingAnswer {
    StandingAnswer {
        answer_id: Uuid::nil(),
        answer_text: text.to_string(),
        points_to: Some(serde_json::json!(["R1"])),
        read_text: None,
        read_error: None,
        read_abstain_reason: None,
    }
}

/// ADDENDUM_1: the dock tells the model a failed analysis FAILED, rather than
/// handing it Marie's failure sentence; a real analysis passes through.
#[test]
fn the_dock_names_a_failed_analysis_and_passes_a_real_one() {
    let mut failed = saved("words");
    failed.read_text =
        Some("Your answer is saved, but the answer analysis didn't come back.".into());
    failed.read_error = Some("the call failed: timeout".into());
    failed.read_abstain_reason = Some("the model could not be reached".into());
    assert_eq!(analysis_for(Some(&failed)), Some(FAILED_ANALYSIS));

    let mut judged = saved("words");
    judged.read_text = Some("Fine. Short, and yours.".into());
    assert_eq!(analysis_for(Some(&judged)), Some("Fine. Short, and yours."));

    assert_eq!(
        analysis_for(Some(&saved("words"))),
        None,
        "no analysis stored"
    );
    assert_eq!(analysis_for(None), None, "no answer");
}

/// (M) A draft, when sent, is what the model discusses — flagged, with no picks.
#[test]
fn a_draft_replaces_the_saved_answer_for_this_message() {
    let answer = discussed_answer(Some("  my new words  "), Some(&saved("old words")));
    assert_eq!(
        answer,
        DiscussedAnswer {
            text: "my new words".to_string(),
            is_draft: true,
            points_to: None
        }
    );
}

/// No draft: the saved answer, with its picks. Neither: the named absence.
#[test]
fn without_a_draft_the_saved_answer_or_the_absence_is_sent() {
    let answer = discussed_answer(None, Some(&saved("old words")));
    assert_eq!(answer.text, "old words");
    assert!(!answer.is_draft);
    assert_eq!(answer.points_to, Some(vec!["R1".to_string()]));
    assert_eq!(discussed_answer(Some("   "), None).text, NO_ANSWER);
}

/// A 200 with nothing in it is a failed call, named; a reply is trimmed.
#[test]
fn an_empty_reply_is_a_failure_not_a_turn() {
    assert_eq!(
        usable_reply("  \n ", "claude-opus-5"),
        Err("claude-opus-5 returned an empty reply".to_string())
    );
    assert_eq!(
        usable_reply(" Lead with R1. ", "claude-opus-5"),
        Ok("Lead with R1.".to_string())
    );
}
