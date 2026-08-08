//! Unit tests for [`super`] — the judging fan-out's pure helpers.
//!
//! Moved out of `theme_scan_judge.rs` on 2026-08-08 (task 2.15 Tier 2): that
//! module sat at 285 non-comment lines against the 300 limit with these tests
//! inline, so the group-judging change had no room. The house pattern every other
//! scan module already follows — a `#[path]`-included sibling — buys it back
//! without touching a single assertion below.

use super::*;
use crate::domain::fact_role::FactRole;

fn instance(id: &str) -> BiasInstance {
    BiasInstance {
        evidence_id: id.to_string(),
        title: String::new(),
        verbatim_quote: None,
        question: None,
        statement_type: None,
        page_number: None,
        pattern_tags: Vec::new(),
        stated_by: None,
        about: Vec::new(),
        document: None,
    }
}

fn outcome(verdict: Result<Verdict, String>) -> JudgeOutcome {
    JudgeOutcome {
        verdict,
        raw_reply: None,
        input_tokens: None,
        output_tokens: None,
    }
}

fn verdict(relevant: bool) -> Verdict {
    Verdict {
        relevant,
        proposed_role: FactRole::Supports,
        reason: "r".to_string(),
        confidence: 0.9,
    }
}

#[test]
fn outcome_bucket_classifies_the_three_live_buckets() {
    // The live progress bucket is the VERDICT-time classification (ruling 2).
    assert_eq!(
        outcome_bucket(&outcome(Ok(verdict(true)))),
        ProgressBucket::Relevant
    );
    assert_eq!(
        outcome_bucket(&outcome(Ok(verdict(false)))),
        ProgressBucket::Irrelevant
    );
    assert_eq!(
        outcome_bucket(&outcome(Err("call failed".to_string()))),
        ProgressBucket::Failed
    );
}

#[test]
fn user_message_uses_unknown_for_missing_speaker_and_document() {
    let msg = build_user_message("the accusation", &instance("ev-1"));
    assert!(msg.contains("Speaker: unknown"));
    assert!(msg.contains("Document: unknown"));
    assert!(msg.contains("the accusation"));
}

#[test]
fn user_message_includes_speaker_document_and_quote() {
    let mut c = instance("ev-2");
    c.verbatim_quote = Some("I never said that".to_string());
    c.stated_by = Some(crate::bias::dto::ActorOption {
        id: "p1".to_string(),
        name: "Marie".to_string(),
        actor_type: "Person".to_string(),
        tagged_statement_count: 0,
    });
    c.document = Some(crate::bias::dto::DocumentRef {
        id: "d1".to_string(),
        title: "Affidavit".to_string(),
        document_type: None,
    });
    let msg = build_user_message("the accusation", &c);
    assert!(msg.contains("Speaker: Marie"));
    assert!(msg.contains("Document: Affidavit"));
    assert!(msg.contains("I never said that"));
}

#[test]
fn user_message_pairs_question_and_answer_when_question_present() {
    // Discovery Q&A pairing: a bare answer is presented WITH its question,
    // so the judge can interpret "Yes" against what was actually asked.
    let mut c = instance("ev-3");
    c.verbatim_quote = Some("Yes".to_string());
    c.question = Some("Did you receive the funds on June 1?".to_string());
    let msg = build_user_message("the accusation", &c);

    assert!(
        msg.contains("Question asked: \"Did you receive the funds on June 1?\""),
        "question line missing: {msg}"
    );
    assert!(
        msg.contains("Answer under review: \"Yes\""),
        "answer line missing: {msg}"
    );
    // The single-quote label must NOT appear in the Q&A shape.
    assert!(
        !msg.contains("Quote: \""),
        "Q&A shape must not use the single-quote label: {msg}"
    );
}

#[test]
fn user_message_uses_single_quote_shape_when_question_absent() {
    // Documentary evidence (no question) keeps the ORIGINAL message shape
    // unchanged — no "Question asked" / "Answer under review" lines.
    let mut c = instance("ev-4");
    c.verbatim_quote = Some("The ledger shows a transfer.".to_string());
    // question stays None (documentary evidence).
    let msg = build_user_message("the accusation", &c);

    assert!(
        msg.contains("Quote: \"The ledger shows a transfer.\""),
        "single-quote shape expected: {msg}"
    );
    assert!(
        !msg.contains("Question asked:"),
        "no question line when question is None: {msg}"
    );
    assert!(
        !msg.contains("Answer under review:"),
        "no answer line when question is None: {msg}"
    );
}

#[test]
fn user_message_treats_empty_question_as_absent() {
    // A `question` property that exists but holds "" (or only whitespace)
    // carries no interpretive value — it must read identically to a missing
    // question (the single-quote shape), not emit an empty `Question asked`.
    let mut c = instance("ev-5");
    c.verbatim_quote = Some("No".to_string());
    c.question = Some("   ".to_string());
    let msg = build_user_message("the accusation", &c);

    assert!(
        msg.contains("Quote: \"No\""),
        "empty question must fall back to single-quote shape: {msg}"
    );
    assert!(
        !msg.contains("Question asked:"),
        "empty question must not produce a question line: {msg}"
    );
}
