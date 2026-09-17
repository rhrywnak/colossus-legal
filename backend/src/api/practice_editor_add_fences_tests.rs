//! Unit tests for the add-a-question fences — the ones a `deck_key` decides.
//!
//! `plan_question` needs an `AppState` (it resolves the attach against the
//! settings snapshot), so what is pinned here is `fence_follows`, which is the
//! whole of the redirect-anchor rule and is pure.

use super::fence_follows;
use crate::repositories::pipeline_repository::practice::PracticeQuestionRecord;
use uuid::Uuid;

/// One George row carrying `key`, which is the only field this fence reads
/// besides `kind`.
fn cross_with_key(key: Option<&str>) -> PracticeQuestionRecord {
    PracticeQuestionRecord {
        id: Uuid::from_u128(7),
        scenario_id: Uuid::nil(),
        side: "george".to_string(),
        text: "a question she was asked".to_string(),
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
        deck_key: key.map(str::to_string),
        kind: "cross".to_string(),
        follows_key: None,
        source_line: None,
        hidden_at: None,
        draft_by: None,
        updated_at: chrono::DateTime::from_timestamp(1_755_000_000, 0)
            .expect("a fixed, valid instant"),
    }
}

/// A redirect can anchor to a HAND-ADDED George question. (M)
///
/// ## The defect this pins (CC_TASK_DEFECT_SWEEP_v1 defect 4)
///
/// This fence resolves `follows` against a cross question's `deck_key`. A
/// hand-added question carried NO key, so it could not be followed at all: the
/// redirect was refused with "no George question in this deck has the key …",
/// and the only way out was to write the question into the deck file by hand.
/// The key is now minted in the reserved `x` namespace at insert
/// (`repositories::pipeline_repository::deck_key_mint`), and this is the fence
/// seeing one.
#[test]
fn a_redirect_can_follow_a_hand_added_question() {
    let deck = vec![cross_with_key(Some("x1"))];

    let follows = fence_follows("redirect", Some("x1"), &deck).expect("a minted key is an anchor");

    assert_eq!(follows.as_deref(), Some("x1"));
}

/// The un-keyed row is exactly what could not be followed before.
#[test]
fn a_redirect_cannot_follow_a_question_that_has_no_key() {
    let deck = vec![cross_with_key(None)];

    let Err(error) = fence_follows("redirect", Some("x1"), &deck) else {
        panic!("a key no question holds must be refused");
    };
    assert!(format!("{error:?}").contains("x1"), "{error:?}");
}

/// And a key nothing in the deck holds is still refused, by name.
///
/// The fence's whole job. Without this, the first test would pass just as well
/// against a fence that accepted anything.
#[test]
fn a_redirect_naming_a_key_no_question_holds_is_refused() {
    let deck = vec![cross_with_key(Some("x1"))];

    let Err(error) = fence_follows("redirect", Some("x2"), &deck) else {
        panic!("a redirect anchored to nothing must be refused");
    };
    assert!(format!("{error:?}").contains("x2"), "{error:?}");
}
