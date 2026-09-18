//! "Discuss with AI", the impure half: load a question's thread, send one message.
//!
//! CC_TASK_QUESTION_CHAT_v1 §2. The pure rules (what the model is told, the cap)
//! live in [`super::practice_discuss`]; the one model-call plumbing lives in
//! [`super::practice_model_call`]; this module only fetches, stores and calls, and
//! turns each distinct failure into its own [`DiscussError`].
//!
//! ## The order of a send, and why
//!
//! 1. Refuse a blank message, an unknown model, a missing question, a full cap —
//!    before anything is stored or spent.
//! 2. STORE the user's message, then call the model — as the read stores the
//!    answer before calling. A slow or failed vendor never loses what was typed.
//! 3. Store the model's reply with its model id and cost.
//!
//! A failed call therefore leaves the user's message in the thread with no reply
//! under it, and the dock says so; sending again asks again.

use std::time::Instant;

use uuid::Uuid;

use crate::auth::AuthUser;
use crate::domain::llm_params::{LlmParamsSpec, ParamValue};
use crate::dto::practice_discussion::{DiscussionPayload, DiscussionRequest};
use crate::repositories::pipeline_repository::practice::PracticeQuestionRecord;
use crate::repositories::pipeline_repository::practice_discussions::{
    insert_turn, list_thread, model_turn_count, standing_answer, NewTurn, TurnRole,
};
use crate::services::practice_discuss::{
    cap_reached, choose_model, compose_discussion_input, discussed_answer, usable_reply,
};
use crate::services::practice_discuss_load::{
    load_discussion, require_question, store, DiscussError,
};
use crate::services::practice_model_call::{
    call_model, elapsed_ms, resolve_model, response_tokens,
};
use crate::services::practice_notes::attribution;
use crate::services::practice_read_gather::gather_payload;
use crate::services::practice_read_payload::build_user_message;
use crate::state::AppState;

/// Store one message, ask the model, store its reply, and return the fresh thread.
///
/// # Errors
/// Each refusal and failure is its own [`DiscussError`] variant — see the module
/// doc for the order they are checked in.
pub async fn send_message(
    state: &AppState,
    user: &AuthUser,
    question_id: Uuid,
    request: &DiscussionRequest,
) -> Result<DiscussionPayload, DiscussError> {
    let text = request.text.trim();
    let (model_id, question) = admit(state, question_id, request, text).await?;

    // Everything the model will be told is gathered BEFORE the user turn is
    // stored, so the new message does not appear twice in its own input.
    let prior = list_thread(&state.pipeline_pool, question_id)
        .await
        .map_err(store("list_thread", question_id))?;
    let (author_id, author_name) = attribution(user);
    let input = discussion_input(state, &question, request, &prior, &author_name, text).await?;
    let (system, resolved) = setup(state, &model_id).await?;

    // The user's words are on disk before the vendor is asked anything.
    let asker = Asker {
        question_id,
        author_id: &author_id,
        author_name: &author_name,
    };
    store_turn(state, &asker, TurnRole::User, None, text, None).await?;
    let (reply, cost) = ask(state, &resolved, &system, &input, &model_id).await?;
    store_turn(
        state,
        &Asker {
            author_name: &resolved.record.display_name,
            ..asker
        },
        TurnRole::Model,
        Some(&model_id),
        &reply,
        Some(cost),
    )
    .await?;

    tracing::info!(
        %question_id, model = %model_id, by = %author_id, ms = cost.ms,
        input_tokens = cost.input_tokens, output_tokens = cost.output_tokens,
        draft = request.draft_answer.is_some(),
        "practice discuss: a reply was stored"
    );
    load_discussion(state, question_id).await
}

/// The refusals, in order, before anything is stored or spent: a blank message,
/// an unknown model, a missing question, a full cap.
async fn admit(
    state: &AppState,
    question_id: Uuid,
    request: &DiscussionRequest,
    text: &str,
) -> Result<(String, PracticeQuestionRecord), DiscussError> {
    if text.is_empty() {
        return Err(DiscussError::BlankText);
    }
    let settings = state.settings.current();
    let read = &settings.practice_read;
    // Unknown ids are refused exactly as `/ask` refuses them: against the chat
    // provider map (GO: the dock never calls a `require_ai` route).
    let model_id = choose_model(
        request.model.as_deref(),
        &read.discuss_default_model,
        |id| state.chat_providers.contains_key(id),
    )
    .map_err(DiscussError::UnknownModel)?;
    let question = require_question(state, question_id).await?;
    let used = model_turn_count(&state.pipeline_pool, question_id)
        .await
        .map_err(store("model_turn_count", question_id))?;
    if cap_reached(used, read.discuss_max_turns) {
        return Err(DiscussError::CapReached {
            question_id,
            max: read.discuss_max_turns,
        });
    }
    Ok((model_id, question))
}

/// Who a turn is filed under.
#[derive(Clone, Copy)]
struct Asker<'a> {
    question_id: Uuid,
    author_id: &'a str,
    author_name: &'a str,
}

/// What one model reply cost.
#[derive(Clone, Copy)]
struct Cost {
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
    ms: i32,
}

/// Store one turn; a model turn carries its model and cost.
async fn store_turn(
    state: &AppState,
    asker: &Asker<'_>,
    role: TurnRole,
    model_id: Option<&str>,
    text: &str,
    cost: Option<Cost>,
) -> Result<(), DiscussError> {
    insert_turn(
        &state.pipeline_pool,
        &NewTurn {
            question_id: asker.question_id,
            author_user_id: asker.author_id,
            author_name: asker.author_name,
            role,
            model_id,
            text,
            input_tokens: cost.and_then(|c| c.input_tokens),
            output_tokens: cost.and_then(|c| c.output_tokens),
            ms: cost.map(|c| c.ms),
        },
    )
    .await
    .map(|_| ())
    .map_err(store(
        if role == TurnRole::User {
            "insert user turn"
        } else {
            "insert model turn"
        },
        asker.question_id,
    ))
}

/// One model call through the shared plumbing; an empty reply is a failure.
async fn ask(
    state: &AppState,
    resolved: &crate::services::practice_model_call::ResolvedModel,
    system: &str,
    input: &str,
    model_id: &str,
) -> Result<(String, Cost), DiscussError> {
    let started = Instant::now();
    let response = call_model(
        state,
        resolved.provider.as_ref(),
        system,
        input,
        &resolved.params,
    )
    .await
    .map_err(|e| DiscussError::Call(format!("{model_id}: {e}")))?;
    let (input_tokens, output_tokens) = response_tokens(&response);
    let cost = Cost {
        input_tokens,
        output_tokens,
        ms: elapsed_ms(started),
    };
    let reply = usable_reply(&response.text, model_id).map_err(DiscussError::Call)?;
    Ok((reply, cost))
}

/// The composed model input: the read's package, the analysis, the thread, the message.
async fn discussion_input(
    state: &AppState,
    question: &PracticeQuestionRecord,
    request: &DiscussionRequest,
    prior: &[crate::repositories::pipeline_repository::practice_discussions::DiscussionTurnRecord],
    author_name: &str,
    text: &str,
) -> Result<String, DiscussError> {
    let saved = standing_answer(&state.pipeline_pool, question.id)
        .await
        .map_err(store("standing_answer", question.id))?;
    let answer = discussed_answer(request.draft_answer.as_deref(), saved.as_ref());
    let payload = gather_payload(
        state,
        question.scenario_id,
        question,
        &answer.text,
        answer.points_to.as_ref(),
    )
    .await?;
    let analysis = saved.as_ref().and_then(|a| a.read_text.as_deref());
    Ok(compose_discussion_input(
        &build_user_message(&payload),
        analysis,
        answer.is_draft,
        prior,
        author_name,
        text,
    ))
}

/// The system prompt and the resolved model, through the one plumbing.
async fn setup(
    state: &AppState,
    model_id: &str,
) -> Result<(String, crate::services::practice_model_call::ResolvedModel), DiscussError> {
    let settings = state.settings.current();
    let path = std::path::Path::new(state.registry.template_dir())
        .join(settings.practice_read.discuss_prompt_file.trim());
    let system = std::fs::read_to_string(&path).map_err(|e| {
        DiscussError::Setup(format!(
            "the discussion prompt at {} is unreadable: {e} — deploy the file, or point \
             practice_discuss_prompt_file at one that is there",
            path.display()
        ))
    })?;
    // Natural variation for a conversation: no temperature is set.
    let spec = LlmParamsSpec {
        temperature: ParamValue::Unset,
        timeout_secs: ParamValue::Unset,
        max_tokens: ParamValue::Set(settings.practice_read.discuss_max_tokens),
    };
    // `practice_discuss_effort` — the dock's own dial, for the reason
    // `practice_model_call::resolve_model` gives.
    let resolved = resolve_model(
        state,
        model_id,
        &spec,
        "practice_discuss_default_model",
        settings.practice_read.discuss_effort,
    )
    .await
    .map_err(DiscussError::Setup)?;
    Ok((system, resolved))
}
