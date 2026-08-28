//! Unit tests for the retry caps, the `retry-after` transit encoding, and the
//! backoff schedule.
//!
//! All pure arithmetic — no clock, no socket, no API call.

use super::*;

#[test]
fn the_shipped_policy_is_zero_general_retries_and_five_free_ones() {
    // Pinned rather than assumed. Raising either default should have to argue
    // with a failing test first: the general cap is zero because two
    // deterministic failures were paid for twice in the week of 2026-08-24, and
    // the rate-limit cap is non-zero ONLY because those retries are free.
    let policy = LlmRetryPolicy::default();
    assert_eq!(policy.max_retries, 0);
    assert_eq!(policy.rate_limit_max_retries, 5);
}

#[test]
fn a_providers_retry_after_survives_the_round_trip_unchanged() {
    // The whole point of the sentinel: a real advice value must come back out
    // byte-identical, including the zero that means "retry immediately".
    for secs in [0u64, 1, 17, 60, 3600] {
        let transit = advice_from_header(Some(secs));
        assert_eq!(
            advice_to_header(transit),
            Some(secs),
            "advice of {secs}s must survive transit"
        );
    }
}

#[test]
fn an_absent_retry_after_is_distinguishable_from_a_zero_one() {
    // The distinction the foreign `PipelineError::RateLimited { u64 }` cannot
    // hold on its own, and the reason the sentinel exists. Collapsing these
    // would make "the provider said retry immediately" and "the provider said
    // nothing" produce the same wait, which is exactly the kind of merged state
    // Standing Rule 1 forbids.
    assert_eq!(advice_to_header(advice_from_header(None)), None);
    assert_eq!(advice_to_header(advice_from_header(Some(0))), Some(0));
    assert_ne!(advice_from_header(None), advice_from_header(Some(0)));
}

#[test]
fn the_sentinel_itself_cannot_be_mistaken_for_advice() {
    // A provider claiming a 584-billion-year wait is not a real case, but the
    // encoding must still be total: it maps onto "no advice" rather than
    // through it, so no input produces an ambiguous transit value.
    assert_eq!(
        advice_to_header(advice_from_header(Some(NO_RETRY_ADVICE))),
        None
    );
}

#[test]
fn an_advised_wait_is_honoured_exactly_and_never_replaced_by_backoff() {
    // Domain note: Anthropic's retry-after states when the token bucket will
    // have room for THIS request. Waiting less guarantees another rejection;
    // substituting our own guess for it would be strictly worse information.
    let advice = advice_from_header(Some(17));
    for attempt in 1..=5 {
        assert_eq!(
            wait_before_retry(advice, attempt),
            Duration::from_secs(17),
            "attempt {attempt} must still honour the provider's own number"
        );
    }
}

#[test]
fn an_unadvised_wait_follows_the_doubling_schedule() {
    // The common shape for a 529, which usually carries no header at all.
    let none = advice_from_header(None);
    let seen: Vec<u64> = (1..=5)
        .map(|a| wait_before_retry(none, a).as_secs())
        .collect();
    assert_eq!(seen, vec![1, 2, 4, 8, 16]);
}

#[test]
fn the_backoff_is_capped_and_cannot_overflow_at_an_absurd_cap() {
    // An operator who sets LLM_RATE_LIMIT_RETRY_MAX to something silly must get
    // a long wait, not a shift past the width of a u64 (a panic in debug, a
    // wrong answer in release).
    let none = advice_from_header(None);
    assert_eq!(
        wait_before_retry(none, 7).as_secs(),
        60,
        "the seventh step would be 64s; the cap holds it at 60"
    );
    for attempt in [8u32, 64, 1000, u32::MAX] {
        assert_eq!(wait_before_retry(none, attempt).as_secs(), 60);
    }
}

#[test]
fn attempt_zero_is_treated_as_the_first_step_rather_than_underflowing() {
    // `attempt` is 1-based by contract, but a caller that passed 0 must get the
    // first step, not a subtraction below zero.
    let none = advice_from_header(None);
    assert_eq!(wait_before_retry(none, 0).as_secs(), 1);
}
