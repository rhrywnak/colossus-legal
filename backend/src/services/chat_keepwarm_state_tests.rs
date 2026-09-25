//! The keep-warm state: a turn and a ping fingerprint the case file the same
//! way, a changed case file fingerprints differently, and a real turn wakes the
//! pinger.

use std::time::Duration as StdDuration;

use colossus_chat::{Message, Role};
use serde_json::json;

use super::*;
use crate::domain::chat_params::QuestionChatParams;
use crate::repositories::pipeline_repository::chat_discussions::CorpusDocument;
use crate::services::chat_case_prefix::build_request;
use crate::services::chat_question_text::package_documents;
use crate::services::chat_question_tools::tools;

fn corpus(order_text: &str) -> Vec<CorpusDocument> {
    vec![
        CorpusDocument {
            id: "d1".into(),
            title: "The brief".into(),
            document_date: None,
            pages: vec![(1, "Page one.".into())],
        },
        CorpusDocument {
            id: "d2".into(),
            title: "The order".into(),
            document_date: None,
            pages: vec![(1, order_text.into())],
        },
    ]
}

/// A real turn's request (question context, a conversation) with its tools.
fn turn_parts(order_text: &str) -> (ChatRequest, Vec<Arc<dyn ChatTool>>) {
    let documents = Arc::new(package_documents(&corpus(order_text)));
    let history = vec![Message {
        role: Role::User,
        content: vec![json!({"type": "text", "text": "What does the order say?"})],
    }];
    let request = build_request(
        &QuestionChatParams::for_test(),
        "prompt".into(),
        "narrative".into(),
        "THE QUESTION: when did you last speak to him?".into(),
        &documents,
        history,
    );
    (
        request,
        tools(documents, "Attempt 1: I do not recall.".into()),
    )
}

/// The ping's request, as `case_file_request` builds it.
fn ping_hash(order_text: &str) -> String {
    let documents = Arc::new(package_documents(&corpus(order_text)));
    let mut request = build_request(
        &QuestionChatParams::for_test(),
        "prompt".into(),
        "narrative".into(),
        String::new(),
        &documents,
        Vec::new(),
    );
    request.tools = tools(documents, String::new())
        .iter()
        .map(|t| t.spec())
        .collect();
    prefix_hash(&build_prewarm_body(&request).expect("documents present"))
}

#[test]
fn a_turn_and_a_ping_fingerprint_the_same_case_file_identically() {
    let (request, turn_tools) = turn_parts("Ordered.");
    let turn = turn_prefix_hash(&request, &turn_tools).expect("documents present");
    assert_eq!(turn, ping_hash("Ordered."));
    assert_eq!(turn.len(), 64, "a SHA-256 in lowercase hex");
}

#[test]
fn a_changed_document_changes_the_fingerprint() {
    assert_ne!(ping_hash("Ordered."), ping_hash("Ordered, and amended."));
}

#[test]
fn record_turn_sets_the_reference_the_ping_will_compare_with() {
    let keepwarm = KeepWarm::default();
    let (request, turn_tools) = turn_parts("Ordered.");
    record_turn(&keepwarm, &request, &turn_tools);
    assert_eq!(
        keepwarm.ledger().prefix_reference,
        Some(ping_hash("Ordered."))
    );
}

#[test]
fn a_turn_with_no_documents_keeps_the_old_reference() {
    let keepwarm = KeepWarm::default();
    keepwarm.touched("older".into());
    let (mut request, turn_tools) = turn_parts("Ordered.");
    request.documents.clear();
    record_turn(&keepwarm, &request, &turn_tools);
    assert_eq!(keepwarm.ledger().prefix_reference.as_deref(), Some("older"));
}

#[tokio::test]
async fn a_real_turn_wakes_the_wait_before_its_deadline() {
    let keepwarm = Arc::new(KeepWarm::default());
    let waiter = Arc::clone(&keepwarm);
    let far = Utc::now() + chrono::Duration::hours(1);
    let handle = tokio::spawn(async move { waiter.wait(far).await });
    tokio::time::sleep(StdDuration::from_millis(20)).await;
    keepwarm.touched("abc".into());
    let woke = tokio::time::timeout(StdDuration::from_secs(5), handle)
        .await
        .expect("the wait ended well before its one-hour deadline")
        .expect("the task did not panic");
    assert_eq!(woke, Woke::Touched);
}

#[tokio::test]
async fn a_touch_while_nobody_waits_is_kept_for_the_next_wait() {
    let keepwarm = KeepWarm::default();
    keepwarm.touched("abc".into());
    let far = Utc::now() + chrono::Duration::hours(1);
    assert_eq!(keepwarm.wait(far).await, Woke::Touched);
}

#[tokio::test]
async fn a_passed_deadline_ends_the_wait_at_once() {
    let keepwarm = KeepWarm::default();
    let past = Utc::now() - chrono::Duration::minutes(1);
    assert_eq!(keepwarm.wait(past).await, Woke::Deadline);
}

#[test]
fn update_changes_the_ledger_and_returns_the_closures_value() {
    let keepwarm = KeepWarm::default();
    let spent = keepwarm.update(|l| {
        l.spend_dollars = 0.5;
        l.spend_dollars
    });
    assert_eq!(spent, 0.5);
    assert_eq!(keepwarm.ledger().spend_dollars, 0.5);
}
