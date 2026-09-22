// Tests for `services::practice_read` — the row a finished read writes.
//
// The attempt loop's own tests (re-requests, corrections, token sums) live
// beside it in `practice_read_attempts_tests.rs`.

use super::{finish, plain_reason_for, stamp};
use crate::services::practice_read_attempts::{Attempts, AttemptsEnd, TokenCost};
use crate::services::practice_read_outcome::ReadOutcome;
use crate::services::practice_read_parse::{ReadReply, ReplyRejection};

const ABSTAIN: &str = "I can't read this one.";
const FAILED: &str = "Your answer is saved, but the answer analysis didn't come back this time.";

fn run(end: AttemptsEnd, attempts: u8) -> Attempts {
    let mut spent = TokenCost::default();
    for _ in 0..attempts {
        spent.add(Some(2000), Some(100));
    }
    Attempts {
        end,
        attempts,
        spent,
    }
}

fn finished(end: AttemptsEnd, attempts: u8) -> ReadOutcome {
    finish(
        "practice_read_prompt_v5.md",
        run(end, attempts),
        ABSTAIN,
        FAILED,
        "claude-opus-5".to_string(),
        900,
    )
}

/// Q1, reversed by Roman: a reply unusable twice shows the ONE failure line.
///
/// The technical cause is on the row for an operator, never on her screen.
#[test]
fn a_reply_rejected_twice_shows_the_failure_line_and_keeps_the_cause_for_operators() {
    let outcome = finished(
        AttemptsEnd::Rejected {
            rejection: ReplyRejection::KeyInProse {
                part: "why".to_string(),
                token: "R2".to_string(),
            },
            raw: "{\"why\": \"R2\"}".to_string(),
        },
        2,
    );
    assert_eq!(outcome.text.as_deref(), Some(FAILED));
    assert!(
        outcome.failed(),
        "the flag the screen and the chat context read"
    );
    assert!(outcome.text.as_deref().is_some_and(|t| !t.contains("R2")));
    assert!(outcome
        .error
        .as_deref()
        .is_some_and(|e| e.contains("R2") && e.contains("why")));
    assert!(outcome
        .abstain_reason
        .as_deref()
        .is_some_and(|r| r.contains("internal labels")));
    assert_eq!(outcome.attempts, Some(2));
    assert_eq!(
        outcome.version.as_deref(),
        Some("practice_read_prompt_v5.md")
    );
    assert_eq!(outcome.input_tokens, Some(4000));
    assert!(outcome.raw_reply.is_some());
    assert!(outcome.parts.is_none());
}

/// Every system-failure arm reads the same line.
#[test]
fn a_call_that_failed_and_a_loop_with_no_attempt_show_the_same_failure_line() {
    let failed = finished(
        AttemptsEnd::CallFailed {
            error: "the call failed: timeout".to_string(),
            last_raw: None,
        },
        1,
    );
    assert_eq!(failed.text.as_deref(), Some(FAILED));
    assert!(failed
        .error
        .as_deref()
        .is_some_and(|e| e.contains("timeout")));

    assert!(failed.failed());
    let none = finished(AttemptsEnd::NoAttempt, 0);
    assert!(none.failed());
    assert_eq!(none.text.as_deref(), Some(FAILED));
    assert_eq!(none.attempts, None, "no call was made, so no attempt count");
}

/// A MODEL decline is not a system fault: the model's own words, as before.
#[test]
fn a_model_decline_still_shows_the_abstain_line_and_the_models_sentence() {
    let outcome = finished(
        AttemptsEnd::Accepted {
            reply: ReadReply::Abstain("That looks like a test entry.".to_string()),
            raw: "{}".to_string(),
            overruns: Vec::new(),
        },
        1,
    );
    assert_eq!(
        outcome.text.as_deref(),
        Some("I can't read this one. That looks like a test entry.")
    );
    assert_ne!(outcome.text.as_deref(), Some(FAILED));
    assert!(!outcome.failed(), "a model decline is not a system failure");
}

/// A read that failed still says what it spent.
///
/// The DEV row of 2026-09-17 is the case: attempt 1 completed, attempt 2 was
/// truncated at the ceiling, and the stored row carried NULL for both token
/// columns — a read costing thousands of tokens that no total would ever
/// count. The cost of the attempts that DID return is not erased by a later
/// one that did not.
#[test]
fn a_failed_call_still_records_what_the_attempts_cost() {
    let mut spent = TokenCost::default();
    spent.add(Some(4436), Some(699));

    let mut outcome = ReadOutcome::default();
    stamp(spent, &mut outcome);

    assert_eq!(outcome.input_tokens, Some(4436));
    assert_eq!(outcome.output_tokens, Some(699));
}

/// And a read that never reached the model reports no spend, not zero spend.
#[test]
fn a_read_that_never_called_reports_no_spend_rather_than_zero() {
    let mut outcome = ReadOutcome::default();
    stamp(TokenCost::default(), &mut outcome);

    assert_eq!(outcome.input_tokens, None);
    assert_eq!(outcome.output_tokens, None);
}

/// A key in the prose has its own operator reason, distinct from the others.
///
/// Since v2.2.1 Marie reads one failure line whatever happened, so this
/// sentence is the ONLY place the row says which of five things it was. Two
/// causes sharing one sentence would be two states with one observable.
#[test]
fn every_rejection_has_its_own_operator_reason_and_key_in_prose_is_named() {
    let key_in_prose = plain_reason_for(&ReplyRejection::KeyInProse {
        part: "why".to_string(),
        token: "R2".to_string(),
    });
    assert!(key_in_prose.contains("internal labels"), "{key_in_prose}");
    let unknown = plain_reason_for(&ReplyRejection::UnknownKey {
        key: "R9".to_string(),
        sent: "R1".to_string(),
    });
    let empty = plain_reason_for(&ReplyRejection::Empty);
    let unparseable = plain_reason_for(&ReplyRejection::Unparseable {
        detail: "x".to_string(),
    });
    for other in [unknown, empty, unparseable] {
        assert_ne!(key_in_prose, other);
    }
}

/// The failure flag's rule, over every kind of stored row (ADDENDUM_1).
#[test]
fn only_a_system_failure_counts_as_a_failed_read() {
    use crate::services::practice_read_outcome::{is_failed_read, MODEL_ABSTAINED_PREFIX};
    // A system failure — including a pre-v2.2.1 one with the old line.
    assert!(is_failed_read(
        Some("the model could not be reached"),
        Some("the call failed: 529")
    ));
    assert!(is_failed_read(Some("an input failed to load"), None));
    // The model's own decline.
    let declined = format!("{MODEL_ABSTAINED_PREFIX}That looks like a test entry.");
    assert!(!is_failed_read(
        Some("That looks like a test entry."),
        Some(&declined)
    ));
    // A judgement, a stored don't-recall, analysis off, a read in flight.
    assert!(!is_failed_read(None, None));
    assert!(!is_failed_read(
        None,
        Some("no read: the answer analysis switch was off, so no model was asked")
    ));
}
