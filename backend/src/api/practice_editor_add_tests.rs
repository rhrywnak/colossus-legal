//! The `after` fence and the placement it guards (task DECK_DRAG_AND_ADD part 3).
//!
//! `post_add_question` itself needs an `AppState` — a graph, pools and a settings
//! snapshot — so what is pinned here is the two pure decisions the handler is
//! built out of, which is where the behaviour a person notices actually lives:
//!
//! * **which refusal** an impossible `after` earns. Roman ruled 400-vs-404 on
//!   2026-09-10 and the difference is not cosmetic: a 404 says the row is not
//!   there, and sends whoever meets it looking for a question somebody deleted.
//! * **that a same-side `after` is accepted**, so the fence cannot quietly refuse
//!   the ordinary case and leave every gap control dead.
//!
//! The placement arithmetic behind it is pinned in
//! `repositories::pipeline_repository::practice_reorder::place_tests`.

use super::*;
use uuid::Uuid;

/// A scenario id distinct from `Uuid::nil()`, so a message that names it is
/// visibly naming THIS scenario rather than a default.
const SCENARIO: Uuid = Uuid::from_u128(0xabcd);

/// One deck row, on a named side. Only the three fields the fence reads matter.
fn question(n: u128, side: &str) -> PracticeQuestionRecord {
    PracticeQuestionRecord {
        id: Uuid::from_u128(n),
        scenario_id: Uuid::nil(),
        side: side.to_string(),
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
        updated_at: chrono::DateTime::from_timestamp(1_755_000_000, 0)
            .expect("a fixed, valid instant"),
    }
}

/// One George row and one of Chuck's — the smallest deck that has two sides.
fn deck() -> Vec<PracticeQuestionRecord> {
    vec![question(1, "george"), question(2, "chuck")]
}

/// A request naming `after`, otherwise the simplest valid add.
fn request(after: Option<Uuid>) -> AddQuestionRequest {
    AddQuestionRequest {
        kind: "cross".to_string(),
        text: "a new question".to_string(),
        tactic: None,
        follows: None,
        watch_for: None,
        source_kind: None,
        source_index: None,
        after,
        at_start: false,
    }
}

/// A request asking for the TOP of the side — the gap above the first row.
fn at_start_request() -> AddQuestionRequest {
    AddQuestionRequest {
        at_start: true,
        ..request(None)
    }
}

/// The plan a `cross` request becomes: George's side.
fn plan() -> AddPlan {
    AddPlan {
        side: "george",
        kind: "cross",
        tactic: None,
        follows: None,
        source_kind: "manual",
        source_ref: None,
    }
}

#[test]
fn no_after_is_the_end_of_the_side() {
    // The path every existing caller takes — the bottom "+ Add a question" box,
    // and every client written before the gap control existed. A fence that
    // refused it, or that read it as anything but the end, would move every
    // question anybody adds.
    assert_eq!(
        fence_after(&request(None), &plan(), &deck(), Uuid::nil())
            .expect("no position is a real request"),
        NewPosition::End
    );
}

#[test]
fn an_after_on_the_same_side_names_that_row() {
    // The gap control's ordinary request. Pinned so the fence cannot be tightened
    // into refusing the case it exists to allow.
    let row = Uuid::from_u128(1);
    assert_eq!(
        fence_after(&request(Some(row)), &plan(), &deck(), Uuid::nil()).expect("a same-side after"),
        NewPosition::After(row)
    );
}

/// The gap above the FIRST row is its own position, and not "no position".
///
/// The task specified `after = none` for that gap while also specifying that an
/// add with no `after` appends — one absent value asked to mean both the very
/// top and the very end. `at_start` is the flag that separates them; without it,
/// pressing the topmost gap would put the question at the bottom of the deck,
/// which is the one place the person pressing it did not point at.
#[test]
fn at_start_is_the_top_of_the_side_and_not_the_end() {
    assert_eq!(
        fence_after(&at_start_request(), &plan(), &deck(), Uuid::nil())
            .expect("the top is a real request"),
        NewPosition::Start
    );
}

/// Both at once is a 400 — the two name different places.
#[test]
fn at_start_together_with_after_is_refused() {
    let both = AddQuestionRequest {
        at_start: true,
        ..request(Some(Uuid::from_u128(1)))
    };
    match fence_after(&both, &plan(), &deck(), Uuid::nil()) {
        Err(AppError::BadRequest { details, .. }) => assert_eq!(details["field"], "at_start"),
        other => panic!("expected a 400, got {other:?}"),
    }
}

/// A row this scenario does not hold is a 404 — the caller named nothing.
#[test]
fn an_after_naming_no_row_in_this_deck_is_a_404() {
    let missing = Uuid::from_u128(404);
    match fence_after(&request(Some(missing)), &plan(), &deck(), SCENARIO) {
        Err(AppError::NotFound { message }) => {
            assert!(
                message.contains(&missing.to_string()),
                "the refusal must name the id that was not found: {message}"
            );
            // …and the scenario, which is what tells a stale tab naming a deleted
            // row apart from a request aimed at the wrong deck.
            assert!(
                message.contains(&SCENARIO.to_string()),
                "the refusal must name the scenario it looked in: {message}"
            );
            // …and what to try instead. A 404 with no next move is a dead end.
            assert!(
                message.contains("omit `after`"),
                "the refusal must say what a valid value looks like: {message}"
            );
        }
        other => panic!("expected a 404, got {other:?}"),
    }
}

/// A row on the OTHER side is a 400 naming the field — not a 404.
///
/// Roman's ruling, and the reason for it: the row IS there. Answering 404 would
/// say it does not exist, which is false, and would send whoever met it hunting
/// for a question somebody had deleted. The request is well-formed and asks for
/// something the deck cannot express — placing a George question under one of
/// Chuck's — which is what 400 means.
#[test]
fn an_after_on_the_other_side_is_a_400_that_names_the_field() {
    let chuck_row = Uuid::from_u128(2);
    match fence_after(&request(Some(chuck_row)), &plan(), &deck(), Uuid::nil()) {
        Err(AppError::BadRequest { message, details }) => {
            assert_eq!(details["field"], "after");
            assert_eq!(details["value"], serde_json::json!(chuck_row));
            // Both sides named, because "wrong side" without saying which two is
            // a refusal a person has to reproduce to understand.
            assert!(
                message.contains("george") && message.contains("chuck"),
                "the refusal must name both sides: {message}"
            );
        }
        other => panic!("expected a 400, got {other:?}"),
    }
}

/// The fence runs BEFORE the transaction opens.
///
/// A source-shape test, because the ordering is invisible to the compiler and it
/// is the difference between a refused add that wrote nothing and one that wrote
/// a row, moved it, and rolled back. `fence_after` is called from the handler,
/// not from `write_question`, and that is what this pins.
#[test]
fn the_after_fence_runs_before_anything_is_written() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/api/practice_editor_add.rs");
    let text = std::fs::read_to_string(path).expect("practice_editor_add.rs is readable");
    let start = text
        .find("pub async fn post_add_question")
        .expect("the handler is present");
    let body = &text[start..];
    let end = body.find("\n}").map(|i| i + 2).unwrap_or(body.len());
    let body = &body[..end];

    let fence = body.find("fence_after(").expect("the fence is called");
    let write = body.find("write_question(").expect("the write is called");
    assert!(
        fence < write,
        "an impossible position must be refused before a row is written: {body}"
    );
}
