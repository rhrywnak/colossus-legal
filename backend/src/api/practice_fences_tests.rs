//! Tests for the two fences that need no database.
//!
//! `fence_not_already_answered` and `refuse_if_practised` both read a pool and
//! are covered the way this repository covers pool-bound reads: their SQL is
//! parsed off disk by `practice_sql_shape` and asserted against the migrations.
//! These two are pure, so they are asserted directly — a fence nothing tests is
//! a fence somebody deletes in a refactor because it "looked redundant".

use uuid::Uuid;

use super::{fence_answer_text, fence_who};
use crate::{dto::practice::AnswerRequest, error::AppError};

/// One answer body, with the two fields the fence reads set as asked.
fn body(answer_text: &str, dont_recall: bool) -> AnswerRequest {
    AnswerRequest {
        session_id: Uuid::from_u128(1),
        question_id: Uuid::from_u128(2),
        answer_text: answer_text.to_string(),
        dont_recall,
        points_to: None,
    }
}

/// The field a `BadRequest`'s details name, or a panic naming what came back.
///
/// Reading `details.field` rather than matching the MESSAGE: the message is
/// prose that may yet move into the wording store, and a test that pins prose
/// fails on an edit that changed nothing.
fn refused_field(result: Result<(), AppError>) -> String {
    match result {
        Err(AppError::BadRequest { details, .. }) => details
            .get("field")
            .and_then(|f| f.as_str())
            .unwrap_or("<no field>")
            .to_string(),
        Err(other) => panic!("expected a 400, got {other:?}"),
        Ok(()) => panic!("expected a refusal, got Ok"),
    }
}

// ── fence_answer_text ───────────────────────────────────────────────────────

/// A typed answer passes, which is the whole normal path.
#[test]
fn an_answer_with_words_in_it_passes() {
    assert!(fence_answer_text(&body("It was about $500.", false)).is_ok());
}

/// An empty box is refused, naming the field.
#[test]
fn an_empty_answer_is_refused() {
    assert_eq!(
        refused_field(fence_answer_text(&body("", false))),
        "answer_text"
    );
}

/// Whitespace is not words. A newline in the box would otherwise write a row
/// that prints blank on Chuck's sheet under her name.
#[test]
fn whitespace_alone_is_refused_too() {
    assert_eq!(
        refused_field(fence_answer_text(&body("   \n\t ", false))),
        "answer_text"
    );
}

/// "I don't recall." is a COMPLETE answer and arrives with an empty box.
///
/// The exemption that makes this fence safe: without it, the one-click control
/// Marie is most likely to need would be the one the fence blocked.
#[test]
fn dont_recall_passes_with_an_empty_box() {
    assert!(fence_answer_text(&body("", true)).is_ok());
    assert!(fence_answer_text(&body("   ", true)).is_ok());
}

// ── fence_who ───────────────────────────────────────────────────────────────

/// The three sides the column permits all pass.
#[test]
fn the_three_permitted_sides_pass() {
    for who in ["george", "chuck", "mixed"] {
        assert!(fence_who(who).is_ok(), "{who} is a permitted side");
    }
}

/// Anything else is a 400 naming the field — not a 500 with a constraint name.
#[test]
fn an_unknown_side_is_refused_as_a_bad_request() {
    assert_eq!(refused_field(fence_who("plaintiff")), "who");
    assert_eq!(refused_field(fence_who("")), "who");
}

/// The refusal carries the offending VALUE, so a log line names what was sent.
#[test]
fn the_refusal_names_the_value_it_saw() {
    let Err(AppError::BadRequest { details, .. }) = fence_who("George") else {
        panic!("a capitalised side is not one of the three");
    };
    assert_eq!(
        details.get("value").and_then(|v| v.as_str()),
        Some("George")
    );
}
