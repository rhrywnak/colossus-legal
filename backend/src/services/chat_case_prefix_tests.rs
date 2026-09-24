//! The case file is identical whether a real turn or a pre-warm builds it.

use colossus_chat::{build_prewarm_body, Role};
use serde_json::json;

use super::*;
use crate::repositories::pipeline_repository::chat_discussions::CorpusDocument;

fn corpus() -> Vec<CorpusDocument> {
    vec![
        CorpusDocument {
            id: "d1".into(),
            title: "The brief".into(),
            document_date: None,
            pages: vec![(1, "Page one.".into()), (2, "Page two.".into())],
        },
        CorpusDocument {
            id: "d2".into(),
            title: "The order".into(),
            document_date: None,
            pages: vec![(1, "Ordered.".into())],
        },
    ]
}

/// The tools a turn offers, as `run_turn` turns them into the request.
fn specs(documents: &Arc<Vec<PackagedDocument>>, answers: &str) -> Vec<colossus_chat::ToolSpec> {
    tools(Arc::clone(documents), answers.to_string())
        .iter()
        .map(|t| t.spec())
        .collect()
}

/// A real turn (with its per-question context, a conversation, and an answer
/// history) and a pre-warm (with none of those) produce the SAME pre-warm body,
/// which means the same cached bytes.
#[test]
fn a_turn_and_a_prewarm_build_the_same_case_file() {
    let chat = QuestionChatParams::for_test();
    let documents = Arc::new(package_documents(&corpus()));
    let history = vec![Message {
        role: Role::User,
        content: vec![json!({"type": "text", "text": "What does the order say?"})],
    }];
    let mut turn = build_request(
        &chat,
        "prompt".into(),
        "narrative".into(),
        "THE QUESTION: when did you last speak to him?".into(),
        &documents,
        history,
    );
    turn.tools = specs(&documents, "Attempt 1: I do not recall.");
    let mut warm = build_request(
        &chat,
        "prompt".into(),
        "narrative".into(),
        String::new(),
        &documents,
        Vec::new(),
    );
    warm.tools = specs(&documents, "");
    assert_eq!(
        build_prewarm_body(&turn).unwrap(),
        build_prewarm_body(&warm).unwrap()
    );
}

/// The answer history changes what a tool returns, never how it is described:
/// the pre-warm may build its tools with an empty one.
#[test]
fn tool_specs_do_not_depend_on_the_answer_history() {
    let documents = Arc::new(package_documents(&corpus()));
    assert_eq!(
        specs(&documents, "Attempt 1: a long answer."),
        specs(&documents, "")
    );
}

/// The request carries the chat's configured model, effort and cache lifetime
/// — the settings the provider renders into the prompt.
#[test]
fn the_request_carries_the_configured_model_settings() {
    let chat = QuestionChatParams::for_test();
    let documents = package_documents(&corpus());
    let r = build_request(
        &chat,
        "p".into(),
        "n".into(),
        String::new(),
        &documents,
        Vec::new(),
    );
    assert_eq!(r.model, chat.model);
    assert_eq!(r.effort.as_deref(), chat.effort.map(|e| e.as_wire()));
    assert_eq!(r.cache_ttl, chat.cache_ttl);
    assert!(r.adaptive_thinking);
    assert_eq!(r.system, vec!["p".to_string(), "n".to_string()]);
    assert_eq!(r.documents.len(), 2);
}
