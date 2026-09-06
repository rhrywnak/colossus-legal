// Tests for `api::scenario_fact_include::include_link`.
//
// Pure validation, and the only guard between a click and a half-written include.
// What is asserted hardest is the direction of each refusal: an include missing
// its accusation must FAIL, because the alternative is a fact silently placed in
// a scenario and invisible to the accusation it was included for — the exact
// duplicate work §2 exists to end.

use super::*;

use crate::dto::FactAction;

fn request(
    action: FactAction,
    allegation_id: Option<&str>,
    stance: Option<CardStance>,
) -> FactActionRequest {
    FactActionRequest {
        action,
        reason: None,
        allegation_id: allegation_id.map(str::to_string),
        stance,
    }
}

/// Every action that is not an include is untouched by this change.
#[test]
fn a_non_include_action_asks_for_no_link() {
    for action in [
        FactAction::Drop,
        FactAction::Undrop,
        FactAction::Defer,
        FactAction::Reopen,
    ] {
        // ## Rust Learning: the payload needs a `let` binding
        //
        // `include_link` returns a borrow OF its argument, so the request cannot
        // be a temporary inside the call — it has to outlive the result it lends
        // the id to. Naming it is the whole fix.
        let payload = request(action, None, None);
        let link = include_link(&payload).expect("only an include is asked for a link");
        assert!(link.is_none(), "{action:?} should carry no link");
    }
}

/// A non-include that DOES carry link fields still asks for nothing.
///
/// The fields are siblings of `action`, not a tagged union, so a client can send
/// them with any action. Reading them on a drop would write a link for a fact the
/// human just removed.
#[test]
fn a_non_include_carrying_link_fields_still_asks_for_no_link() {
    let payload = request(
        FactAction::Drop,
        Some("alleg-41"),
        Some(CardStance::Supports),
    );
    let link = include_link(&payload).expect("a drop is not validated as an include");
    assert!(link.is_none());
}

#[test]
fn an_include_with_both_halves_is_accepted() {
    let payload = request(
        FactAction::Include,
        Some("alleg-41"),
        Some(CardStance::Rebuts),
    );
    let (allegation_id, stance) = include_link(&payload)
        .expect("a complete include is accepted")
        .expect("an include carries a link");
    assert_eq!(allegation_id, "alleg-41");
    assert_eq!(stance, CardStance::Rebuts);
}

/// The id is TRIMMED, because a pasted id carries the whitespace around it and an
/// untrimmed key matches no allegation row.
#[test]
fn the_accusation_id_is_trimmed() {
    let payload = request(
        FactAction::Include,
        Some("  alleg-41\n"),
        Some(CardStance::Supports),
    );
    let (allegation_id, _) = include_link(&payload)
        .expect("surrounding whitespace is not a refusal")
        .expect("an include carries a link");
    assert_eq!(allegation_id, "alleg-41");
}

#[test]
fn an_include_with_no_accusation_is_refused_and_names_both_fields() {
    let err = include_link(&request(
        FactAction::Include,
        None,
        Some(CardStance::Supports),
    ))
    .expect_err("an include without its accusation is refused");
    let AppError::BadRequest { message, details } = err else {
        panic!("a missing field is a 400, not a 500");
    };
    assert!(message.contains("accusation"), "{message}");
    assert_eq!(
        details["fields"],
        serde_json::json!(["allegation_id", "stance"])
    );
}

#[test]
fn an_include_with_no_stance_is_refused() {
    let payload = request(FactAction::Include, Some("alleg-41"), None);
    let err = include_link(&payload).expect_err("an include without a stance is refused");
    let AppError::BadRequest { details, .. } = err else {
        panic!("a missing field is a 400, not a 500");
    };
    assert_eq!(
        details["fields"],
        serde_json::json!(["allegation_id", "stance"])
    );
}

/// A BLANK id is refused separately, and the refusal names the one field at fault.
///
/// It arrives as `Some("")` from a form the human tabbed through, so the
/// both-fields message above would be wrong — the stance is there.
#[test]
fn an_include_with_a_blank_accusation_is_refused_by_name() {
    let payload = request(FactAction::Include, Some("   "), Some(CardStance::Supports));
    let err = include_link(&payload).expect_err("a blank accusation is not an accusation");
    let AppError::BadRequest { message, details } = err else {
        panic!("a blank field is a 400, not a 500");
    };
    assert!(message.contains("accusation"), "{message}");
    assert_eq!(details["field"], serde_json::json!("allegation_id"));
}
