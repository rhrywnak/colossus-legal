//! Sending a message to the question chat — the half that runs BEFORE the reply
//! streams, and so can still answer with a plain HTTP status.
//!
//! Order matters and is the point of this module:
//!
//! 1. refuse what can be refused without spending anything — blank text, a
//!    thread that is not yours, no engine, an unusable model, the reply cap, a
//!    missing prompt or narrative, a package too large for the model;
//! 2. only then STORE her message — so a model failure never loses what she typed;
//! 3. hand back a [`PreparedTurn`] that [`crate::services::chat_question_stream`]
//!    runs in its own task.

use std::sync::Arc;

use colossus_chat::{ChatBackend, ChatRequest, ChatTool, Message, Role};
use serde_json::json;
use uuid::Uuid;

use crate::domain::chat_params::QuestionChatParams;
use crate::repositories::pipeline_repository::chat_discussions::{
    append_message, assistant_reply_count, get_or_create_discussion, list_messages,
    DiscussionRecord, NewMessage, ANCHOR_QUESTION,
};
use crate::repositories::pipeline_repository::models::{get_model_by_id, LlmModelRecord};
use crate::repositories::pipeline_repository::PipelineRepoError;
use crate::services::chat_case_prefix::{build_request, read_template};
use crate::services::chat_model_check::unusable_reason;
use crate::services::chat_prefix_size::{package_size, PackageSize, PrefixParts};
use crate::services::chat_question_context::{render_attempts, render_context};
use crate::services::chat_question_error::{store, ChatRunError};
use crate::services::chat_question_gather::{display_name, gather};
use crate::services::chat_question_text::PackagedDocument;
use crate::services::chat_question_tools::tools;
use crate::state::AppState;

/// Everything the streaming task needs, owned — it outlives the request.
pub struct PreparedTurn {
    pub question_id: Uuid,
    pub owner: String,
    pub discussion_id: Uuid,
    pub user_seq: i32,
    pub model: String,
    pub backend: Arc<dyn ChatBackend>,
    pub request: ChatRequest,
    pub tools: Vec<Arc<dyn ChatTool>>,
    pub documents: Arc<Vec<PackagedDocument>>,
    pub max_rounds: u32,
    /// The resolved configuration, stored with the turn's result.
    pub run_config: serde_json::Value,
}

/// Validate, store her message, and prepare the model call.
///
/// # Errors
/// See [`ChatRunError`]; each has its own status.
pub async fn prepare_turn(
    state: &AppState,
    question_id: Uuid,
    owner: &str,
    viewer: &str,
    text: &str,
) -> Result<PreparedTurn, ChatRunError> {
    let settings = state.settings.current();
    let chat = &settings.question_chat;
    let text = text.trim();
    let (backend, model_row) = refuse_early(state, question_id, owner, viewer, text).await?;
    let gathered = gather(state, question_id, viewer).await?;
    let discussion = open_thread_under_cap(state, question_id, owner, chat.max_turns).await?;
    let prompt = read_template(state, "the question chat's prompt", &chat.prompt_file).await?;
    let narrative = read_template(state, "the case narrative", &chat.narrative_file).await?;
    let context = render_context(&gathered.context);
    let system = vec![prompt.clone(), narrative.clone()];
    // The probe closure runs only when the prefix has not been counted yet; see
    // `package_size`. Everything it needs is cloned INSIDE it, so a cache hit —
    // which is every turn after the first — copies no document text at all.
    // The two lines that know what a document is; `chat_prefix_size` does not.
    let prefix = PrefixParts {
        system: &system,
        documents: gathered
            .documents
            .iter()
            .map(|d| {
                (
                    d.block.title.as_str(),
                    d.block.context.as_deref(),
                    d.block.text.as_str(),
                )
            })
            .collect(),
    };
    let size = package_size(
        backend.as_ref(),
        &state.chat_prefix_size,
        &prefix,
        &context,
        usize::try_from(chat.chars_per_token).unwrap_or(1),
        || {
            build_request(
                chat,
                prompt.clone(),
                narrative.clone(),
                context.clone(),
                &gathered.documents,
                Vec::new(),
            )
        },
    )
    .await;
    check_size(size, model_row.as_ref(), chat)?;

    let user_seq = store_user_message(state, question_id, discussion.id, text).await?;
    let history = replay_history(state, question_id, discussion.id).await?;
    let answers = render_attempts(&gathered.context);
    let documents = Arc::new(gathered.documents);
    Ok(PreparedTurn {
        question_id,
        owner: owner.to_string(),
        discussion_id: discussion.id,
        user_seq,
        model: chat.model.clone(),
        backend,
        request: build_request(chat, prompt, narrative, context, &documents, history),
        tools: tools(Arc::clone(&documents), answers),
        documents,
        max_rounds: chat.max_tool_rounds,
        run_config: run_config(chat, size),
    })
}

/// The owner's thread (created on first use), refused by name at the reply cap.
async fn open_thread_under_cap(
    state: &AppState,
    question_id: Uuid,
    owner: &str,
    max_turns: u32,
) -> Result<DiscussionRecord, ChatRunError> {
    let pool = &state.pipeline_pool;
    let discussion =
        get_or_create_discussion(pool, ANCHOR_QUESTION, &question_id.to_string(), owner)
            .await
            .map_err(store("get_or_create_discussion", question_id))?;
    let replies = assistant_reply_count(pool, discussion.id)
        .await
        .map_err(store("assistant_reply_count", question_id))?;
    if replies >= i64::from(max_turns) {
        return Err(ChatRunError::CapReached { max: max_turns });
    }
    Ok(discussion)
}

/// What this turn ran under, stored on its last assistant row — settings change,
/// and a turn read back later must say which values applied to IT.
pub fn run_config(chat: &QuestionChatParams, size: PackageSize) -> serde_json::Value {
    json!({
        // What the size guard saw, and whether the provider counted it or this
        // build estimated it — the counterpart of `cache_ttl` below.
        "package_tokens": size.tokens(),
        "package_tokens_provenance": size.provenance(),
        "model": chat.model,
        "max_tokens": chat.max_tokens,
        "effort": chat.effort.map(|e| e.as_wire()),
        "cache_ttl": format!("{:?}", chat.cache_ttl),
        "max_tool_rounds": chat.max_tool_rounds,
        "compaction_trigger_tokens": chat.compaction_trigger_tokens,
        "context_headroom_tokens": chat.context_headroom_tokens,
        "prompt_file": chat.prompt_file,
        "narrative_file": chat.narrative_file,
        // The four values rendered INTO the package the model reads.
        "witness_display_name": chat.witness_display_name,
        "asker_cross": chat.asker_cross,
        "asker_direct": chat.asker_direct,
        "asker_redirect": chat.asker_redirect,
        "ai_display_name": chat.ai_display_name,
        // The size guard's divisor, which now sizes only the per-question tail:
        // a refusal is diagnosable from the row alone.
        "chars_per_token": chat.chars_per_token,
        // The reply cap this turn was admitted under.
        "max_turns": chat.max_turns,
    })
}

/// Store her message — BEFORE the model is asked, so a failure never loses it.
async fn store_user_message(
    state: &AppState,
    question_id: Uuid,
    discussion_id: Uuid,
    text: &str,
) -> Result<i32, ChatRunError> {
    let message = NewMessage {
        role: "user",
        content: json!([{"type": "text", "text": text}]),
        rendered_text: Some(text.to_string()),
        ..NewMessage::default()
    };
    append_message(&state.pipeline_pool, discussion_id, &message)
        .await
        .map_err(store("append_message", question_id))
}

/// Everything refusable before a single row is read or written.
async fn refuse_early(
    state: &AppState,
    question_id: Uuid,
    owner: &str,
    viewer: &str,
    text: &str,
) -> Result<(Arc<dyn ChatBackend>, Option<LlmModelRecord>), ChatRunError> {
    let settings = state.settings.current();
    if text.is_empty() {
        return Err(ChatRunError::BlankText);
    }
    if owner != viewer {
        return Err(ChatRunError::NotOwner {
            owner: display_name(&settings, owner),
        });
    }
    let backend = state.chat_engine.clone().ok_or(ChatRunError::EngineOff)?;
    let model_row = get_model_by_id(&state.pipeline_pool, &settings.question_chat.model)
        .await
        .map_err(|e| store("get_model_by_id", question_id)(PipelineRepoError::from(e)))?;
    if let Some(reason) = unusable_reason(model_row.as_ref()) {
        return Err(ChatRunError::ModelUnusable(reason));
    }
    Ok((backend, model_row))
}

/// Refuse, by name and number, a package the model's context cannot hold with
/// the configured headroom left for the conversation and the reply.
///
/// `size` is [`package_size`]'s answer — the provider's own count of the fixed
/// prefix plus an estimate of the per-question tail, or, when the provider could
/// not be asked, arithmetic that has already been warned about. Before v2.2.2
/// this function did the arithmetic itself at 3 characters per token and
/// under-counted a citation-enabled corpus by 1.6× (CC_TASK_CHAT_COST_FIX_v1 §3).
fn check_size(
    size: PackageSize,
    model: Option<&LlmModelRecord>,
    chat: &QuestionChatParams,
) -> Result<(), ChatRunError> {
    let estimate = usize::try_from(size.tokens()).unwrap_or(usize::MAX);
    // A row with no recorded context size is treated as holding nothing: an
    // unknown limit refuses rather than guessing large.
    // best-effort: a context size that is absent, negative, or too large for this
    // platform is read as 0 — i.e. the guard REFUSES, by name, rather than guess.
    let limit = model
        .and_then(|m| m.max_context_tokens)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(0);
    let headroom = usize::try_from(chat.context_headroom_tokens).unwrap_or(usize::MAX);
    if estimate.saturating_add(headroom) > limit {
        return Err(ChatRunError::ContextTooLarge {
            estimate,
            limit,
            model: chat.model.clone(),
            provenance: size.provenance(),
        });
    }
    Ok(())
}

/// The stored thread as the model must see it again: every message verbatim,
/// EXCEPT failed assistant turns (a failure marker has no content to replay).
async fn replay_history(
    state: &AppState,
    question_id: Uuid,
    discussion_id: Uuid,
) -> Result<Vec<Message>, ChatRunError> {
    let rows = list_messages(&state.pipeline_pool, discussion_id)
        .await
        .map_err(store("list_messages", question_id))?;
    Ok(rows
        .into_iter()
        .filter(|m| m.failure.is_none())
        .map(|m| Message {
            role: if m.role == "assistant" {
                Role::Assistant
            } else {
                Role::User
            },
            content: m.content.as_array().cloned().unwrap_or_default(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The snapshot names every value that shapes a turn, as the store holds it.
    #[test]
    fn the_run_config_records_what_the_turn_ran_under() {
        let c = run_config(
            &QuestionChatParams::for_test(),
            PackageSize::Measured(370_001),
        );
        assert_eq!(c["package_tokens"], 370_001);
        assert_eq!(c["package_tokens_provenance"], "measured");
        assert_eq!(c["model"], "claude-opus-5");
        assert_eq!(c["max_tokens"], 16000);
        assert_eq!(c["effort"], "high");
        assert_eq!(c["cache_ttl"], "OneHour");
        assert_eq!(c["max_tool_rounds"], 6);
        assert_eq!(c["compaction_trigger_tokens"], 700000);
        assert_eq!(c["context_headroom_tokens"], 80000);
        assert_eq!(c["prompt_file"], "question_chat_prompt_v1.md");
        assert_eq!(c["narrative_file"], "case_narrative_v1.md");
        assert_eq!(c["witness_display_name"], "Marie");
        assert_eq!(c["asker_cross"], "opposing counsel, on cross-examination");
        assert_eq!(
            c["asker_direct"],
            "the witness's own lawyer, on direct examination"
        );
        assert_eq!(c["asker_redirect"], "the witness's own lawyer, on redirect");
        assert_eq!(c["ai_display_name"], "The AI");
        assert_eq!(c["chars_per_token"], 3);
        assert_eq!(c["max_turns"], 200);
    }
}
