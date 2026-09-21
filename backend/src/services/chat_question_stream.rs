//! Running a prepared chat turn in its own task: stream it, then store it.
//!
//! ## Rust Learning: `tokio::spawn` + an unbounded channel as a stream bridge
//!
//! The HTTP handler returns a server-sent-event STREAM immediately; the model
//! call runs in a separate task that owns everything it needs (a clone of
//! `AppState`, the `PreparedTurn`). The two talk through
//! `tokio::sync::mpsc::unbounded_channel`: the task `send`s events, the response
//! body reads them. Two consequences worth knowing:
//!
//! - `UnboundedSender::send` is synchronous and never waits, so it can be called
//!   from inside the engine's plain `Fn` callbacks.
//! - If the browser goes away, the receiver drops and `send` starts failing — but
//!   the TASK carries on to the end and stores the reply. Her question is
//!   answered whether or not she stayed to watch.

use std::time::Instant;

use colossus_chat::{run_turn, ChatError, ChatEvent, ChatOutcome, ChatTransportError, Role};
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::dto::chat_discussion::MessageDto;
use crate::repositories::pipeline_repository::chat_discussions::{
    append_message, list_messages, mark_read, NewMessage,
};
use crate::services::chat_question_gather::display_name;
use crate::services::chat_question_run::PreparedTurn;
use crate::services::chat_question_view::{cards_for, message_dto};
use crate::state::AppState;

// STRUCTURAL: the failure vocabulary the screen maps to its sentences
// (`practice_chat_refused` / `_truncated` / `_stalled` / `_send_failed`).
pub const FAILURE_REFUSED: &str = "refused";
pub const FAILURE_TRUNCATED: &str = "truncated";
pub const FAILURE_STALLED: &str = "stalled";
pub const FAILURE_FAILED: &str = "failed";

/// One server-sent event, as the browser reads it (`event:` name + JSON `data:`).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum StreamEvent {
    /// Her message is stored, at this seq.
    Accepted { seq: i32 },
    /// A fragment of the reply.
    Delta { text: String },
    /// The model is reading the record (`started`) or finished reading it.
    Tool { state: &'static str },
    /// The reply is stored; these are the new messages to show.
    Done { messages: Vec<MessageDto> },
    /// The reply failed and a named marker is stored. `failure` is the vocabulary
    /// above; `detail` is for the log line, not the screen.
    Failed {
        failure: String,
        detail: String,
        messages: Vec<MessageDto>,
    },
}

impl StreamEvent {
    /// The `event:` name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Accepted { .. } => "accepted",
            Self::Delta { .. } => "delta",
            Self::Tool { .. } => "tool",
            Self::Done { .. } => "done",
            Self::Failed { .. } => "failed",
        }
    }

    /// The `data:` payload (the variant's fields as a JSON object).
    pub fn data(&self) -> Value {
        match serde_json::to_value(self) {
            Ok(Value::Object(mut m)) => m.remove("data").unwrap_or(Value::Null),
            // Unreachable for this derive; said out loud rather than panicking.
            _ => json!({"error": "event could not be serialized"}),
        }
    }
}

/// Start the turn; return the event stream it feeds.
pub fn spawn_turn(state: AppState, turn: PreparedTurn) -> UnboundedReceiver<StreamEvent> {
    let (tx, rx) = unbounded_channel();
    emit(&tx, StreamEvent::Accepted { seq: turn.user_seq });
    tokio::spawn(async move { run_and_store(state, turn, tx).await });
    rx
}

/// Send one event; a closed receiver (the browser left) is noted once per event
/// at debug level and otherwise changes nothing — see the module doc.
fn emit(tx: &UnboundedSender<StreamEvent>, event: StreamEvent) {
    if tx.send(event).is_err() {
        tracing::debug!("question chat: the browser has gone; the reply is still being stored");
    }
}

async fn run_and_store(state: AppState, turn: PreparedTurn, tx: UnboundedSender<StreamEvent>) {
    let started = Instant::now();
    let sink = |e: ChatEvent| match e {
        ChatEvent::TextDelta(text) => emit(&tx, StreamEvent::Delta { text }),
        ChatEvent::ToolStarted { .. } => emit(&tx, StreamEvent::Tool { state: "started" }),
        ChatEvent::ToolFinished { .. } => emit(&tx, StreamEvent::Tool { state: "finished" }),
    };
    let result = run_turn(
        &*turn.backend,
        turn.request.clone(),
        &turn.tools,
        turn.max_rounds,
        &sink,
    )
    .await;
    let ms = i32::try_from(started.elapsed().as_millis()).unwrap_or(i32::MAX);
    let outcome = match result {
        Ok(outcome) => store_outcome(&state, &turn, &outcome, ms).await,
        Err(e) => store_failure(&state, &turn, &e, ms).await,
    };
    match outcome {
        Ok((failure, detail)) => {
            let messages = new_messages(&state, &turn).await;
            match failure {
                None => emit(&tx, StreamEvent::Done { messages }),
                Some(f) => emit(
                    &tx,
                    StreamEvent::Failed {
                        failure: f,
                        detail,
                        messages,
                    },
                ),
            }
        }
        Err(detail) => emit(
            &tx,
            StreamEvent::Failed {
                failure: FAILURE_FAILED.into(),
                detail,
                messages: Vec::new(),
            },
        ),
    }
}

/// Store every message the turn produced. Returns the failure marker the LAST
/// one carries (`refused` / `truncated`), or `None` for a clean reply.
async fn store_outcome(
    state: &AppState,
    turn: &PreparedTurn,
    outcome: &ChatOutcome,
    ms: i32,
) -> Result<(Option<String>, String), String> {
    // STRUCTURAL: the provider's stop_reason vocabulary (wire protocol).
    let failure = match outcome.stop_reason.as_str() {
        "refusal" => Some(FAILURE_REFUSED.to_string()),
        "max_tokens" => Some(FAILURE_TRUNCATED.to_string()),
        _ => None,
    };
    let detail = failure.as_ref().map_or_else(String::new, |f| {
        format!("the model stopped with {} ({f})", outcome.stop_reason)
    });
    let last = outcome.messages.len().saturating_sub(1);
    for (i, m) in outcome.messages.iter().enumerate() {
        let tail = (i == last).then(|| Tail {
            failure: failure.clone(),
            detail: (!detail.is_empty()).then(|| detail.clone()),
            ms,
        });
        let row = row_for(turn, outcome, i, m, tail);
        append_message(&state.pipeline_pool, turn.discussion_id, &row)
            .await
            .map_err(|e| log_store_failure(turn, "append_message", &e.to_string()))?;
    }
    tracing::info!(question_id = %turn.question_id, owner = %turn.owner, rounds = outcome.rounds,
        input = ?outcome.usage.input_tokens, output = ?outcome.usage.output_tokens,
        cache_read = ?outcome.usage.cache_read_input_tokens, cache_write = ?outcome.usage.cache_creation_input_tokens,
        rejected_citations = outcome.rejected.len(), ms, stop = %outcome.stop_reason,
        "question chat: reply stored");
    Ok((failure, detail))
}

/// What only the LAST message of a turn carries: its failure, and (below) the
/// turn's totals and configuration.
struct Tail {
    failure: Option<String>,
    detail: Option<String>,
    ms: i32,
}

/// One produced message as the row to store.
fn row_for(
    turn: &PreparedTurn,
    outcome: &ChatOutcome,
    index: usize,
    m: &colossus_chat::Message,
    tail: Option<Tail>,
) -> NewMessage {
    if m.role != Role::Assistant {
        return NewMessage {
            role: "user",
            content: Value::Array(m.content.clone()),
            ..NewMessage::default()
        };
    }
    let cards = cards_for(
        outcome.citations.get(&index).map_or(&[][..], Vec::as_slice),
        &turn.documents,
    );
    let text = joined_text(&m.content);
    let failed = tail.as_ref().is_some_and(|t| t.failure.is_some());
    let base = match tail {
        Some(t) => NewMessage {
            stop_reason: Some(outcome.stop_reason.clone()),
            failure: t.failure,
            failure_detail: t.detail,
            run_config: Some(turn.run_config.clone()),
            ..usage_row(outcome, t.ms)
        },
        None => NewMessage::default(),
    };
    NewMessage {
        role: "assistant",
        content: Value::Array(m.content.clone()),
        rendered_text: (!failed && !text.trim().is_empty()).then_some(text),
        citations: (!cards.is_empty()).then(|| json!(cards)),
        model: Some(turn.model.clone()),
        ..base
    }
}

/// Token counts and timing ride the LAST assistant row of the turn (the turn's
/// totals); earlier rows of the same turn carry none — summing them would double.
fn usage_row(outcome: &ChatOutcome, ms: i32) -> NewMessage {
    // best-effort: a count past i32::MAX (two billion tokens in one turn) cannot
    // occur under any model's context window; it would be stored as "not
    // reported" rather than wrapped to a negative number.
    let n = |v: Option<u64>| v.and_then(|x| i32::try_from(x).ok());
    NewMessage {
        input_tokens: n(outcome.usage.input_tokens),
        output_tokens: n(outcome.usage.output_tokens),
        cache_creation_tokens: n(outcome.usage.cache_creation_input_tokens),
        cache_read_tokens: n(outcome.usage.cache_read_input_tokens),
        ms: Some(ms),
        ..NewMessage::default()
    }
}

/// Store a named failure row for a turn that produced no reply — with the full
/// sentence and the configuration, so the row says what failed and under what.
async fn store_failure(
    state: &AppState,
    turn: &PreparedTurn,
    error: &ChatError,
    ms: i32,
) -> Result<(Option<String>, String), String> {
    let failure = match error {
        ChatError::Transport(ChatTransportError::IdleTimeout { .. }) => FAILURE_STALLED,
        _ => FAILURE_FAILED,
    };
    let detail = error.to_string();
    tracing::error!(question_id = %turn.question_id, owner = %turn.owner, %failure, error = %detail,
        "question chat: the reply failed; her message is kept and a failure marker is stored");
    let row = NewMessage {
        role: "assistant",
        content: json!([]),
        model: Some(turn.model.clone()),
        failure: Some(failure.to_string()),
        failure_detail: Some(detail.clone()),
        run_config: Some(turn.run_config.clone()),
        ms: Some(ms),
        ..NewMessage::default()
    };
    append_message(&state.pipeline_pool, turn.discussion_id, &row)
        .await
        .map_err(|e| log_store_failure(turn, "append_message (failure marker)", &e.to_string()))?;
    Ok((Some(failure.to_string()), detail))
}

fn log_store_failure(turn: &PreparedTurn, operation: &str, detail: &str) -> String {
    tracing::error!(question_id = %turn.question_id, owner = %turn.owner, operation, error = detail,
        "question chat: the reply could not be stored");
    format!("{operation} failed: {detail}")
}

/// The prose of an assistant message: its text blocks, joined.
pub fn joined_text(content: &[Value]) -> String {
    content
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|b| b.get("text").and_then(Value::as_str))
        .collect()
}

/// The messages stored after her message, as the thread shows them, and her
/// read mark moved past them (she watched them arrive).
async fn new_messages(state: &AppState, turn: &PreparedTurn) -> Vec<MessageDto> {
    let settings = state.settings.current();
    let tz = &settings.practice_read.case_timezone;
    let owner_name = display_name(&settings, &turn.owner);
    let rows = match list_messages(&state.pipeline_pool, turn.discussion_id).await {
        Ok(rows) => rows,
        Err(e) => {
            log_store_failure(turn, "list_messages (after the reply)", &e.to_string());
            return Vec::new();
        }
    };
    if let Some(last) = rows.last() {
        if let Err(e) = mark_read(
            &state.pipeline_pool,
            turn.discussion_id,
            &turn.owner,
            last.seq,
        )
        .await
        {
            log_store_failure(turn, "mark_read", &e.to_string());
        }
    }
    rows.iter()
        .filter(|m| m.seq > turn.user_seq)
        .filter_map(|m| {
            let author = if m.role == "assistant" {
                settings.question_chat.ai_display_name.as_str()
            } else {
                owner_name.as_str()
            };
            message_dto(m, author, tz)
        })
        .collect()
}

#[cfg(test)]
mod tests {
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
}
