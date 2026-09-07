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

// ─── The Matrix half (integration ruling R1) ─────────────────────────────────

/// The verdict an include writes is `Keep`, and the ids go in the right slots.
///
/// Both ids are `&str`, so nothing but this test stands between a correct call
/// and one that files the verdict against the accusation instead of the item.
#[test]
fn an_include_keeps_the_item_on_the_matrix() {
    let at = chrono::Utc::now();
    let write = keep_write(
        "doc-tighe:evidence:b49268dd",
        "doc-complaint:allegation:45984d77",
        "marie",
        at,
    );

    assert_eq!(write.evidence_id, "doc-tighe:evidence:b49268dd");
    assert_eq!(write.allegation_id, "doc-complaint:allegation:45984d77");
    assert_eq!(write.ruling, MatrixRuling::Keep);
    assert_eq!(write.ruled_by, "marie");
    assert_eq!(write.written_at, at);
}

/// THE RULE THIS TEST EXISTS FOR. `Keep` whichever way the fact cuts.
///
/// The stance says what the statement DOES to the accusation; the Matrix verdict
/// says a human read the pairing and it belongs on the page. A fact Marie
/// includes BECAUSE it rebuts the accusation is exactly the item the Matrix must
/// keep — `Remove` there would strike the evidence the include was for.
///
/// The link's `cut` is asserted alongside so the two stay visibly independent:
/// the ruling is the same for both stances and the cut is not.
#[test]
fn the_verdict_is_keep_for_both_stances_while_the_cut_differs() {
    use crate::domain::link_cut::LinkCut;

    let at = chrono::Utc::now();
    for stance in [CardStance::Supports, CardStance::Rebuts] {
        let write = keep_write("node", "alleg", "marie", at);
        assert_eq!(
            write.ruling,
            MatrixRuling::Keep,
            "{stance:?} must still keep the item"
        );
    }

    assert_eq!(CardStance::Supports.to_link_cut(), LinkCut::Against);
    assert_eq!(CardStance::Rebuts.to_link_cut(), LinkCut::Supports);
}

/// The Matrix note stays empty, because nobody typed one.
///
/// `note` is a sentence a human wrote on the Matrix page. Composing one here
/// ("included from the scenario page") would put words into a record whose whole
/// value is that they are a person's own — the actor and the instant already say
/// how the row got there.
#[test]
fn the_include_writes_no_matrix_note() {
    let write = keep_write("node", "alleg", "marie", chrono::Utc::now());
    assert!(write.note.is_none());
}
