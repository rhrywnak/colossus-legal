//! Tool-loop tests against a scripted backend: no socket, no tokens.

use std::sync::Mutex;

use super::*;
use crate::accumulate::AssistantMessage;
use crate::request::{CacheTtl, ChatDocument, ToolSpec};

/// Returns canned replies in order and records every body it was sent.
struct Scripted {
    replies: Mutex<Vec<AssistantMessage>>,
    bodies: Mutex<Vec<Value>>,
}

impl Scripted {
    fn new(mut replies: Vec<AssistantMessage>) -> Self {
        replies.reverse();
        Self {
            replies: Mutex::new(replies),
            bodies: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl ChatBackend for Scripted {
    async fn call(
        &self,
        body: &Value,
        on_text: &(dyn Fn(String) + Send + Sync),
    ) -> Result<AssistantMessage, ChatTransportError> {
        self.bodies.lock().unwrap().push(body.clone());
        let reply = self.replies.lock().unwrap().pop().expect("script ran out");
        on_text(reply.text());
        Ok(reply)
    }

    async fn count_tokens(&self, _body: &Value) -> Result<u64, ChatTransportError> {
        unreachable!("the tool loop never counts; the caller's size guard does")
    }

    async fn prewarm(&self, _body: &Value) -> Result<crate::usage::Usage, ChatTransportError> {
        unreachable!("the tool loop never pre-warms")
    }
}

struct Echo;

#[async_trait::async_trait]
impl ChatTool for Echo {
    fn name(&self) -> &str {
        "echo"
    }
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "echo".into(),
            description: "echo".into(),
            input_schema: json!({"type":"object"}),
        }
    }
    async fn run(&self, input: Value) -> Result<String, String> {
        match input.get("fail") {
            Some(_) => Err("asked to fail".into()),
            None => Ok(format!("echo:{input}")),
        }
    }
}

fn reply(content: Vec<Value>, stop: &str) -> AssistantMessage {
    AssistantMessage {
        content,
        stop_reason: stop.into(),
        usage: Usage {
            input_tokens: Some(10),
            output_tokens: Some(2),
            ..Usage::default()
        },
    }
}

fn tool_use(id: &str, name: &str, input: Value) -> Value {
    json!({"type":"tool_use","id":id,"name":name,"input":input})
}

fn request() -> ChatRequest {
    ChatRequest {
        model: "m".into(),
        max_tokens: 10,
        system: vec!["s".into()],
        documents: vec![ChatDocument {
            title: "D".into(),
            context: None,
            text: "The grass is green.".into(),
        }],
        context: "c".into(),
        history: vec![Message {
            role: Role::User,
            content: vec![json!({"type":"text","text":"why?"})],
        }],
        tools: vec![],
        effort: None,
        adaptive_thinking: false,
        compaction_trigger_tokens: None,
        cache_ttl: CacheTtl::FiveMinutes,
    }
}

fn tools() -> Vec<Arc<dyn ChatTool>> {
    vec![Arc::new(Echo)]
}

async fn run(backend: &Scripted, max: u32) -> (Result<ChatOutcome, ChatError>, Vec<ChatEvent>) {
    let seen = Mutex::new(Vec::new());
    let sink = |e: ChatEvent| seen.lock().unwrap().push(e);
    let out = run_turn(backend, request(), &tools(), max, &sink).await;
    (out, seen.into_inner().unwrap())
}

#[tokio::test]
async fn parallel_tool_calls_come_back_in_one_user_turn() {
    let backend = Scripted::new(vec![
        reply(
            vec![
                tool_use("a", "echo", json!({"n":1})),
                tool_use("b", "echo", json!({"n":2})),
            ],
            "tool_use",
        ),
        reply(vec![json!({"type":"text","text":"done"})], "end_turn"),
    ]);
    let (out, events) = run(&backend, 5).await;
    let out = out.unwrap();
    assert_eq!(out.rounds, 2);
    assert_eq!(out.messages.len(), 3);
    let results = &out.messages[1].content;
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["tool_use_id"], "a");
    assert_eq!(results[1]["tool_use_id"], "b");
    assert_eq!(out.usage.input_tokens, Some(20));
    // The second call's body carries the whole turn so far, verbatim.
    let bodies = backend.bodies.lock().unwrap();
    let msgs = bodies[1]["messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 4, "package, question, tool_use, tool_results");
    assert_eq!(bodies[1]["tools"][0]["name"], "echo");
    assert!(events.contains(&ChatEvent::TextDelta("done".into())));
    assert!(events.contains(&ChatEvent::ToolFinished {
        name: "echo".into(),
        ok: true
    }));
}

#[tokio::test]
async fn a_failing_tool_is_reported_to_the_model_as_is_error() {
    let backend = Scripted::new(vec![
        reply(
            vec![tool_use("a", "echo", json!({"fail":true}))],
            "tool_use",
        ),
        reply(vec![json!({"type":"text","text":"ok"})], "end_turn"),
    ]);
    let (out, events) = run(&backend, 5).await;
    let out = out.unwrap();
    assert_eq!(out.messages[1].content[0]["is_error"], true);
    assert!(events.contains(&ChatEvent::ToolFinished {
        name: "echo".into(),
        ok: false
    }));
}

#[tokio::test]
async fn an_unoffered_tool_is_an_error_result_not_a_dead_turn() {
    let backend = Scripted::new(vec![
        reply(
            vec![tool_use("a", "delete_everything", json!({}))],
            "tool_use",
        ),
        reply(vec![json!({"type":"text","text":"sorry"})], "end_turn"),
    ]);
    let out = run(&backend, 5).await.0.unwrap();
    let r = &out.messages[1].content[0];
    assert_eq!(r["is_error"], true);
    assert!(r["content"].as_str().unwrap().contains("delete_everything"));
}

#[tokio::test]
async fn the_round_bound_is_a_named_error() {
    let backend = Scripted::new(vec![
        reply(vec![tool_use("a", "echo", json!({}))], "tool_use"),
        reply(vec![tool_use("b", "echo", json!({}))], "tool_use"),
    ]);
    let (out, _) = run(&backend, 2).await;
    assert!(matches!(out, Err(ChatError::ToolRoundsExceeded { max: 2 })));
}

#[tokio::test]
async fn refusal_and_max_tokens_end_the_turn_without_running_tools() {
    for stop in ["refusal", "max_tokens"] {
        let backend = Scripted::new(vec![reply(vec![tool_use("a", "echo", json!({}))], stop)]);
        let (out, events) = run(&backend, 5).await;
        let out = out.unwrap();
        assert_eq!(out.stop_reason, stop);
        assert_eq!(out.messages.len(), 1);
        assert!(!events
            .iter()
            .any(|e| matches!(e, ChatEvent::ToolStarted { .. })));
    }
}

#[tokio::test]
async fn an_unknown_stop_reason_is_a_named_error() {
    let backend = Scripted::new(vec![reply(vec![], "pause_turn")]);
    assert!(
        matches!(run(&backend, 5).await.0, Err(ChatError::UnexpectedStop(s)) if s == "pause_turn")
    );
}

#[tokio::test]
async fn citations_are_verified_and_a_forged_one_is_dropped() {
    let good = json!({"type":"char_location","cited_text":"The grass is green.","document_index":0,
        "start_char_index":0,"end_char_index":19});
    let forged = json!({"type":"char_location","cited_text":"The grass is blue.","document_index":0,
        "start_char_index":0,"end_char_index":18});
    let backend = Scripted::new(vec![reply(
        vec![json!({"type":"text","text":"green","citations":[good, forged]})],
        "end_turn",
    )]);
    let out = run(&backend, 5).await.0.unwrap();
    let kept = &out.citations[&0];
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].1.quoted_text, "The grass is green.");
    assert_eq!(out.rejected.len(), 1);
}
