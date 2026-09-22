// Tests for `services::practice_read_attempts` — the loop, driven by recorded
// replies instead of a provider.
//
// Each test hands `run_attempts` a closure that plays back canned replies and
// RECORDS every user message it was asked to send. That record is the proof the
// corrective re-request needs: what attempt 2 actually carried.

use std::cell::RefCell;
use std::collections::BTreeSet;

use colossus_extract::LlmResponse;

use super::*;
use crate::services::practice_read_corrections::split_prompt;

const USER: &str = "THE PAYLOAD";

fn rules() -> ReadRules<'static> {
    ReadRules {
        max_words_call: 12,
        max_words_why: 55,
        max_words_pointer: 20,
        max_pointers: 3,
        max_words_after_fine: 6,
        fine_token: "Fine.",
    }
}

fn citable() -> BTreeSet<String> {
    ["P1", "P2", "R1", "R2", "S1", "S2"]
        .iter()
        .map(|k| (*k).to_string())
        .collect()
}

fn v5_corrections() -> Corrections {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let file =
        std::fs::read_to_string(root.join("extraction_templates/practice_read_prompt_v5.md"))
            .expect("v5 is on disk");
    split_prompt(&file)
        .expect("v5 is complete")
        .corrections
        .expect("v5 carries corrections")
}

fn reply(text: &str) -> LlmResponse {
    LlmResponse {
        text: text.to_string(),
        input_tokens: Some(2000),
        output_tokens: Some(100),
    }
}

/// Play `replies` back in order, recording each message sent. `Err` entries are
/// calls that never returned.
async fn replay(
    corrections: Option<&Corrections>,
    replies: Vec<Result<LlmResponse, String>>,
) -> (Attempts, Vec<String>) {
    let citable = citable();
    let ctx = AttemptRules {
        rules: rules(),
        citable: &citable,
        corrections,
        model_id: "recorded",
    };
    let sent = RefCell::new(Vec::new());
    let queue = RefCell::new(replies.into_iter());
    let run = run_attempts(&ctx, USER, |message: String| {
        sent.borrow_mut().push(message);
        let next = queue
            .borrow_mut()
            .next()
            .expect("the test supplied enough replies");
        async move { next }
    })
    .await;
    (run, sent.into_inner())
}

const KEY_IN_WHY: &str = r#"{"call": "You conceded the fees.", "why": "R2 says the property fight drove the fees.", "keys": ["R2"]}"#;
const CLEAN: &str = r#"{"call": "You conceded the fees.", "why": "The fee order says the property fight drove them.", "keys": ["R2"]}"#;

/// Journey 4: "R2" in `why` twice → re-requested once, then abstained.
#[tokio::test]
async fn key_in_prose_re_requests_once_then_abstains() {
    let c = v5_corrections();
    let (run, sent) = replay(Some(&c), vec![Ok(reply(KEY_IN_WHY)), Ok(reply(KEY_IN_WHY))]).await;

    assert_eq!(run.attempts, 2);
    assert_eq!(sent.len(), 2, "one re-request, never a third call");
    match run.end {
        AttemptsEnd::Rejected { rejection, raw } => {
            assert_eq!(
                rejection,
                ReplyRejection::KeyInProse {
                    part: "why".to_string(),
                    token: "R2".to_string()
                }
            );
            assert_eq!(raw, KEY_IN_WHY, "the last reply is kept for diagnosis");
        }
        other => panic!("expected Rejected, got {other:?}"),
    }
    // Both attempts' cost, not the last one's.
    assert_eq!(run.spent.input, Some(4000));
    assert_eq!(run.spent.output, Some(200));
}

/// The architect's addition: attempt 2 carries the rejected reply AND the
/// one-sentence correction naming what was wrong.
#[tokio::test]
async fn attempt_two_carries_the_rejected_reply_and_the_correction() {
    let c = v5_corrections();
    let (_, sent) = replay(Some(&c), vec![Ok(reply(KEY_IN_WHY)), Ok(reply(CLEAN))]).await;

    assert_eq!(sent[0], USER, "attempt 1 is the payload alone");
    let second = &sent[1];
    assert!(
        second.starts_with(USER),
        "the payload is kept whole: {second}"
    );
    assert!(
        second.contains(KEY_IN_WHY),
        "its own reply is sent back: {second}"
    );
    assert!(
        second.contains("internal label, R2, in `why`"),
        "the correction names the token and the part: {second}"
    );
}

/// A clean second reply is accepted, and the row says it took two attempts.
#[tokio::test]
async fn a_corrected_second_reply_is_accepted() {
    let c = v5_corrections();
    let (run, _) = replay(Some(&c), vec![Ok(reply(KEY_IN_WHY)), Ok(reply(CLEAN))]).await;

    assert_eq!(run.attempts, 2);
    match run.end {
        AttemptsEnd::Accepted {
            reply: ReadReply::Parts(parts),
            ..
        } => {
            assert_eq!(parts.keys, vec!["R2"]);
            assert!(crate::services::practice_read_parse::find_key_token(
                &parts.why,
                crate::services::practice_read_parse::KEY_FAMILIES
            )
            .is_none());
        }
        other => panic!("expected accepted parts, got {other:?}"),
    }
}

/// Each ruled rejection sends ITS correction, not a generic one.
#[tokio::test]
async fn an_unknown_key_sends_the_unknown_key_correction() {
    let c = v5_corrections();
    let invented = r#"{"call": "Name the letter.", "keys": ["R9"]}"#;
    let (_, sent) = replay(Some(&c), vec![Ok(reply(invented)), Ok(reply(CLEAN))]).await;
    assert!(sent[1].contains("Your reply cited R9"), "{}", sent[1]);
}

/// With no corrections section (v4 — the rollback), the re-request is plain.
#[tokio::test]
async fn without_corrections_the_re_request_is_the_plain_one() {
    let (run, sent) = replay(None, vec![Ok(reply(KEY_IN_WHY)), Ok(reply(CLEAN))]).await;
    assert_eq!(sent, vec![USER.to_string(), USER.to_string()]);
    assert!(matches!(run.end, AttemptsEnd::Accepted { .. }));
}

/// An overrun is not a rejection: the plain re-request, as before v2.2.1.
#[tokio::test]
async fn an_overrun_re_requests_plainly() {
    let c = v5_corrections();
    let long = r#"{"call": "one two three four five six seven eight nine ten eleven twelve thirteen", "keys": []}"#;
    let (_, sent) = replay(Some(&c), vec![Ok(reply(long)), Ok(reply(CLEAN))]).await;
    assert_eq!(sent, vec![USER.to_string(), USER.to_string()]);
}

/// A call that never returns ends the loop, keeping the earlier unusable reply.
#[tokio::test]
async fn a_failed_call_after_a_bad_reply_keeps_the_bad_reply() {
    let c = v5_corrections();
    let (run, _) = replay(
        Some(&c),
        vec![Ok(reply(KEY_IN_WHY)), Err("timeout".to_string())],
    )
    .await;
    assert_eq!(run.attempts, 2);
    match run.end {
        AttemptsEnd::CallFailed { error, last_raw } => {
            assert!(error.contains("timeout"), "{error}");
            assert_eq!(last_raw.as_deref(), Some(KEY_IN_WHY));
        }
        other => panic!("expected CallFailed, got {other:?}"),
    }
    assert_eq!(run.spent.input, Some(2000), "attempt 1's cost stands");
}

/// One answer is sent to the model AT MOST twice.
///
/// The re-request is bounded, and the bound is arithmetic rather than a dial.
/// A `MAX_ATTEMPTS` raised on a Settings page would let one typed answer become
/// an unbounded spend; a `MAX_ATTEMPTS` of 1 would silently retire the
/// re-request rule, and every formatting slip would cost Marie her coaching.
#[test]
fn one_answer_is_never_sent_to_the_model_more_than_twice() {
    assert_eq!(MAX_ATTEMPTS, 2);
}

/// Two attempts cost what two attempts cost.
#[test]
fn a_re_requested_read_records_what_both_attempts_cost() {
    let mut spent = TokenCost::default();
    spent.add(Some(2100), Some(180));
    spent.add(Some(2100), Some(240));

    assert_eq!(spent.input, Some(4200));
    assert_eq!(spent.output, Some(420));
}

/// An unreported count is not a free call.
///
/// A provider that reports no tokens must leave the running total alone.
/// Folding `None` in as zero would let one silent attempt make a two-call answer
/// look like a one-call answer.
#[test]
fn an_unreported_token_count_never_reads_as_zero() {
    assert_eq!(accumulate(None, None), None, "never reported stays unknown");
    assert_eq!(accumulate(Some(2100), None), Some(2100));
    assert_eq!(accumulate(None, Some(2100)), Some(2100));
    // And it never wraps into a negative cost.
    assert_eq!(accumulate(Some(i32::MAX), Some(10)), Some(i32::MAX));
}
