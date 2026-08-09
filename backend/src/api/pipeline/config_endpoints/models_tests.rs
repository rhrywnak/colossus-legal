// Tests for the models admin endpoints.
//
// Only the pure validation helper is reachable without a database — the five
// handlers take `State<AppState>` and a live `PgPool`. That helper is the one
// piece of NEW logic in this module that decides something, and it decides the
// thing the 2026-08-09 incident turned on.
//
// A sibling file rather than an inline module for the usual reason: the parent is
// close to the Rule-17 ceiling and its subject (HTTP shape) is distinct from
// this one (what a valid capability token is).

use super::*;
use crate::domain::llm_params::{TEMPERATURE_MODE_TOKEN_OMIT, TEMPERATURE_MODE_TOKEN_ZERO_OK};

/// A capability token nobody recognises is refused AT THE WRITE.
///
/// ## Domain note: why refusing later is not good enough
///
/// `TemperatureMode::from_optional_token` already rejects an unknown token — but
/// it rejects at CALL time, as a provider-construction error, in the middle of a
/// scan somebody is paying for. A bad token written through this endpoint would
/// sit in the registry looking settled until the next run died. On 2026-08-09 a
/// row that merely said NOTHING about its capability cost 104 judge calls in five
/// seconds; a row that says something wrong is the same failure with a longer fuse.
#[test]
fn an_unknown_temperature_mode_is_refused_before_it_reaches_the_registry() {
    let Err(error) = validate_temperature_mode(Some("zero_ok")) else {
        panic!("a near-miss of a real token must not be accepted");
    };

    // The operator has to be able to act on it: which field, what they sent, and
    // what would have worked.
    let AppError::BadRequest { message, details } = error else {
        panic!("an operator's typo is a 400, not a 500");
    };
    assert!(message.contains("zero_ok"), "{message}");
    assert!(
        message.contains(TEMPERATURE_MODE_TOKEN_ZERO_OK),
        "{message}"
    );
    assert!(message.contains(TEMPERATURE_MODE_TOKEN_OMIT), "{message}");
    assert_eq!(details["field"], "temperature_mode");
}

/// Both real tokens pass, and so does saying nothing.
#[test]
fn the_two_recorded_capabilities_and_an_untouched_column_are_all_accepted() {
    for token in [TEMPERATURE_MODE_TOKEN_ZERO_OK, TEMPERATURE_MODE_TOKEN_OMIT] {
        assert!(
            validate_temperature_mode(Some(token)).is_ok(),
            "{token} is a token the resolver itself recognises"
        );
    }

    // ANTI-VACUITY, and a behaviour in its own right. `None` means the request
    // did not mention temperature at all, which the COALESCE update reads as
    // "leave the column alone" — every PUT that edits a display name would fail
    // if absence were treated as an invalid token.
    assert!(validate_temperature_mode(None).is_ok());
}

/// The endpoint validates against the SAME list the resolver parses.
///
/// Two hand-maintained lists would drift, and the drift would show up as a token
/// this endpoint accepts and the provider then refuses at call time — the exact
/// late failure the test above exists to prevent, reintroduced from the other side.
#[test]
fn every_accepted_token_is_one_the_resolver_can_parse() {
    for token in TEMPERATURE_MODE_TOKENS {
        assert!(
            validate_temperature_mode(Some(token)).is_ok(),
            "{token} is offered to operators but refused by the write path"
        );
    }
    assert_eq!(
        TEMPERATURE_MODE_TOKENS.len(),
        2,
        "a third capability would need its own dropdown option and its own \
         resolution rule — it must not arrive silently through this list"
    );
}
