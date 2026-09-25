//! Tests for `chat_question_stream`, moved out of the module when the
//! keep-warm hook took it to Rule 17's limit (CC_TASK_CACHE_KEEPWARM_v1 R3).

use super::*;

#[test]
fn events_serialize_as_name_and_data() {
    let e = StreamEvent::Delta { text: "Hel".into() };
    assert_eq!(e.name(), "delta");
    assert_eq!(e.data(), json!({"text": "Hel"}));
    let a = StreamEvent::Accepted { seq: 7 };
    assert_eq!((a.name(), a.data()), ("accepted", json!({"seq": 7})));
    let t = StreamEvent::Tool { state: "started" };
    assert_eq!((t.name(), t.data()), ("tool", json!({"state": "started"})));
    let d = StreamEvent::Done { messages: vec![] };
    assert_eq!((d.name(), d.data()), ("done", json!({"messages": []})));
    let f = StreamEvent::Failed {
        failure: "stalled".into(),
        detail: "d".into(),
        messages: vec![],
    };
    assert_eq!(f.name(), "failed");
    assert_eq!(f.data()["failure"], "stalled");
}

#[test]
fn joined_text_skips_non_text_blocks() {
    let content = vec![
        json!({"type": "thinking", "thinking": "x"}),
        json!({"type": "text", "text": "A "}),
        json!({"type": "tool_use", "id": "t"}),
        json!({"type": "text", "text": "B"}),
    ];
    assert_eq!(joined_text(&content), "A B");
}
