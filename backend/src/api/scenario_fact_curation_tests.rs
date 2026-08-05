//! Pure-mapping tests for the two curation writes (task 2.13).
//!
//! Split into their own file under the house `#[path = "..._tests.rs"]` pattern
//! so the handler module stays inside the 300-line limit (Rule 17) — the same
//! split `scenario_cards` and `scenario_gather` already use.

use super::*;
use crate::domain::fact_tier::FactTier;
use crate::dto::scenario_curation::{MoveFactRequest, SetTierRequest};
#[test]
fn each_refusal_maps_to_its_own_status() {
    // The three refusals are three different things a human must be told
    // apart: their fact is gone, their neighbour is gone, or there is no room.
    // Collapsing any pair would produce a message that does not fit what
    // happened.
    assert!(matches!(
        move_refusal_to_app_error(MoveRefusal::DraggedNotHere, "ev-1"),
        AppError::NotFound { .. }
    ));
    assert!(matches!(
        move_refusal_to_app_error(MoveRefusal::NeighbourNotHere, "ev-1"),
        AppError::Conflict { .. }
    ));
    assert!(matches!(
        move_refusal_to_app_error(MoveRefusal::GapExhausted, "ev-1"),
        AppError::Conflict { .. }
    ));
}

#[test]
fn the_refusal_text_reaches_the_client_verbatim() {
    // The human is the only one who can act on any of these, so the words must
    // survive the mapping rather than being replaced by a generic failure.
    let refusal = MoveRefusal::GapExhausted;
    let expected = refusal.to_string();
    match move_refusal_to_app_error(refusal, "ev-1") {
        AppError::Conflict { message, details } => {
            assert_eq!(message, expected);
            assert_eq!(details["graph_node_id"], "ev-1");
        }
        other => panic!("gap exhaustion must be a 409, got {other:?}"),
    }
}

#[test]
fn a_tier_body_rejects_an_unknown_token_at_the_parse_boundary() {
    let good: SetTierRequest =
        serde_json::from_value(json!({ "tier": "carries" })).expect("a defined tier parses");
    assert_eq!(good.tier, FactTier::Carries);
    let bad: Result<SetTierRequest, _> = serde_json::from_value(json!({ "tier": "critical" }));
    assert!(
        bad.is_err(),
        "an undefined tier must be a 400, not a default"
    );
}

#[test]
fn a_move_body_accepts_either_neighbour_being_absent() {
    // Top, bottom and only-fact drops are all legal and all shaped differently.
    let top: MoveFactRequest =
        serde_json::from_value(json!({ "before": "ev-2" })).expect("a top drop parses");
    assert_eq!(top.after, None);
    assert_eq!(top.before.as_deref(), Some("ev-2"));
    let alone: MoveFactRequest = serde_json::from_value(json!({})).expect("an only-fact drop");
    assert_eq!(alone.after, None);
    assert_eq!(alone.before, None);
}

#[test]
fn a_move_body_rejects_a_typoed_key() {
    // `deny_unknown_fields`: "aftr" must be a loud 400, not a drop at the top.
    let bad: Result<MoveFactRequest, _> = serde_json::from_value(json!({ "aftr": "ev-2" }));
    assert!(bad.is_err(), "a typo'd neighbour key must not parse");
}
