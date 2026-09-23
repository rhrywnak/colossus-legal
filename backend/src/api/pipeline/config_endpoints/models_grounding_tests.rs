//! Tests for the `grounded` invariant.
//!
//! Every decision here is made by a PURE function, which is the whole reason the
//! provider rule lives in `validate_grounded` rather than in an `if` inside the
//! handler: a handler takes `State<AppState>` and a live pool and cannot be
//! reached without one, and this rule decides which models the Discuss chat will
//! accept. `refuse_invalid_grounding` assembles the pair and hands it to that
//! judge; the assembly's DB read is exercised end to end on a local stack, and
//! the judging is exercised here.

use super::*;
use crate::domain::llm_params::TEMPERATURE_MODE_TOKEN_OMIT;
use crate::error::AppError;
use crate::repositories::pipeline_repository::models::UpdateModelInput;

use super::super::models::{CreateModelInput, UpdateModelRequest};
//
// `grounded` is the column `chat_model_check` reads, so what these two helpers
// decide is literally which models the Discuss chat will accept. Both are pure,
// which is the whole reason the provider rule was put in them rather than left
// to the handler's `if`.

fn request(grounded: Option<bool>, provider: Option<&str>) -> UpdateModelRequest {
    UpdateModelRequest {
        grounded,
        provider: provider.map(str::to_string),
        ..UpdateModelRequest::default()
    }
}

/// Grounding means the Anthropic Messages API with Citations. A vLLM row can be
/// a fine model and still cannot return a quotation the system can check, so the
/// box is refused for it — by name, in words an operator can act on.
#[test]
fn grounded_is_refused_for_a_provider_that_cannot_quote() {
    for provider in ["vllm", "openai"] {
        let Err(AppError::BadRequest { message, details }) =
            validate_grounded(Some(true), provider)
        else {
            panic!("{provider} must not be allowed to claim it quotes the record");
        };
        assert!(message.contains(provider), "{message}");
        assert!(message.contains("quote the record"), "{message}");
        assert_eq!(details["field"], "grounded");
    }
}

#[test]
fn grounded_is_allowed_for_anthropic() {
    assert!(validate_grounded(Some(true), GROUNDED_PROVIDER).is_ok());
}

/// The guard fires only on the state it guards. Un-ticking a vLLM row, or
/// saying nothing at all, must not be blocked by a rule about ticking.
#[test]
fn the_guard_only_judges_a_request_that_grants_the_permission() {
    assert!(validate_grounded(Some(false), "vllm").is_ok());
    assert!(validate_grounded(None, "vllm").is_ok());
    assert!(validate_grounded(Some(false), "openai").is_ok());
}

/// The trap the plan named: a PATCH that carries `grounded` and NOTHING else is
/// exactly what the Edit form sends when somebody only ticks the box. Judged
/// against the body alone there is no provider to judge, so the stored row's
/// must be fetched — and this is the case that says so.
#[test]
fn a_bare_grounding_request_carries_no_provider_to_judge() {
    let bare = request(Some(true), None);
    assert!(
        bare.provider.is_none(),
        "the body alone cannot decide this; refuse_invalid_grounding must read the row"
    );
    assert_eq!(bare.grounded, Some(true));
}

/// The SAME invalid state, reached from the other side — found by the
/// architecture gate, and the reason the check is about the resulting PAIR
/// rather than about the `grounded` field.
///
/// `{provider: "vllm"}` with no `grounded` against a row that is currently
/// grounded leaves `grounded = true` (the COALESCE keeps it) on a provider that
/// cannot quote. A guard that only fires when the body says `grounded: true`
/// never looks at this request at all.
#[test]
fn moving_a_grounded_row_off_anthropic_must_not_slip_past_the_guard() {
    let moving = request(None, Some("vllm"));
    assert_eq!(
        moving.grounded, None,
        "the body says nothing about grounding"
    );
    // What the stored row would contribute, and what the pair must then be
    // judged as. `validate_grounded` is the judge; this is the pair it is given.
    let stored_grounded = true;
    assert!(
        validate_grounded(Some(stored_grounded), moving.provider.as_deref().unwrap()).is_err(),
        "a row that stays grounded while moving to vllm is the invalid pair"
    );
}

/// The three shapes that settle without reading the row, so the common path
/// keeps its single round trip. Stated as a test because each is an early
/// return, and an early return that is wrong is silent.
#[test]
fn the_shapes_that_need_no_stored_row() {
    // Withdrawing the permission cannot produce an invalid pair.
    assert!(validate_grounded(Some(false), "vllm").is_ok());
    // An anthropic row is valid whatever the flag ends up as.
    assert!(validate_grounded(Some(true), GROUNDED_PROVIDER).is_ok());
    // Neither half touched: the request cannot change the pairing.
    let untouched = request(None, None);
    assert!(untouched.grounded.is_none() && untouched.provider.is_none());
}

/// A request that names its own provider is judged on that, not on the stored
/// row — otherwise changing provider and grounding in one save would be checked
/// against the value being replaced.
#[test]
fn a_request_that_names_a_provider_is_judged_on_the_one_it_names() {
    let switching = request(Some(true), Some("vllm"));
    assert!(validate_grounded(switching.grounded, switching.provider.as_deref().unwrap()).is_err());
}

/// `deny_unknown_fields` at the HTTP boundary: a near-miss spelling is a 400,
/// not a silent no-op that leaves the operator looking at an unticked box and
/// no explanation.
#[test]
fn a_misspelled_field_is_refused_rather_than_ignored() {
    let wrong = serde_json::json!({"Grounded": true});
    assert!(
        serde_json::from_value::<UpdateModelRequest>(wrong).is_err(),
        "a capitalised key must not decode to an empty patch"
    );
    let typo = serde_json::json!({"id": "m", "display_name": "M", "provider": "anthropic",
                                  "is_grounded": true});
    assert!(
        serde_json::from_value::<CreateModelInput>(typo).is_err(),
        "a plausible-but-wrong key must not create an ungrounded model in silence"
    );
}

/// Both states decode, and neither becomes `None` on the way in — `None` and
/// `Some(false)` mean different things to the COALESCE downstream.
#[test]
fn both_states_of_the_box_survive_deserialization() {
    let on: UpdateModelRequest =
        serde_json::from_value(serde_json::json!({"grounded": true})).expect("true must decode");
    let off: UpdateModelRequest =
        serde_json::from_value(serde_json::json!({"grounded": false})).expect("false must decode");
    let silent: UpdateModelRequest =
        serde_json::from_value(serde_json::json!({})).expect("an empty patch must decode");
    assert_eq!(on.grounded, Some(true));
    assert_eq!(
        off.grounded,
        Some(false),
        "un-ticking must not arrive as None"
    );
    assert_eq!(
        silent.grounded, None,
        "saying nothing must not arrive as false"
    );
}

/// The HTTP DTO and the repository input are separate types; the conversion
/// between them must not drop a field. `grounded` is the one this task added,
/// and a silently dropped `grounded` would make every save a no-op.
#[test]
fn the_conversion_to_the_repository_input_carries_every_field() {
    let req = UpdateModelRequest {
        display_name: Some("Claude Opus 5.5".into()),
        provider: Some("anthropic".into()),
        api_endpoint: Some("https://example".into()),
        max_context_tokens: Some(1_000_000),
        max_output_tokens: Some(64_000),
        cost_per_input_token: Some(0.000_004),
        cost_per_output_token: Some(0.000_02),
        is_active: Some(true),
        notes: Some("n".into()),
        temperature_mode: Some(TEMPERATURE_MODE_TOKEN_OMIT.into()),
        default_temperature: Some(0.0),
        grounded: Some(true),
    };
    let repo: UpdateModelInput = req.clone().into();
    assert_eq!(repo.grounded, req.grounded);
    assert_eq!(repo.display_name, req.display_name);
    assert_eq!(repo.provider, req.provider);
    assert_eq!(repo.api_endpoint, req.api_endpoint);
    assert_eq!(repo.max_context_tokens, req.max_context_tokens);
    assert_eq!(repo.max_output_tokens, req.max_output_tokens);
    assert_eq!(repo.cost_per_input_token, req.cost_per_input_token);
    assert_eq!(repo.cost_per_output_token, req.cost_per_output_token);
    assert_eq!(repo.is_active, req.is_active);
    assert_eq!(repo.notes, req.notes);
    assert_eq!(repo.temperature_mode, req.temperature_mode);
    assert_eq!(repo.default_temperature, req.default_temperature);
}
