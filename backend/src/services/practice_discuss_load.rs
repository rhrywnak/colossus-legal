//! "Discuss with AI", the read side: the error vocabulary, and loading a thread.
//!
//! CC_TASK_QUESTION_CHAT_v1. Split from `practice_discuss_run` under Rule 17 — the
//! seam is read versus write: this module fetches a question's thread, its model
//! list and its cap, and names every failure; `practice_discuss_run` sends.

use uuid::Uuid;

use crate::domain::scenario_code::scenario_code;
use crate::dto::practice_discussion::{DiscussModelDto, DiscussionPayload};
use crate::repositories::pipeline_repository::models::list_active_models;
use crate::repositories::pipeline_repository::practice::{
    get_question, list_deck, PracticeQuestionRecord,
};
use crate::repositories::pipeline_repository::practice_discussions::{list_thread, TurnRole};
use crate::repositories::pipeline_repository::scenario_store::get_scenario;
use crate::repositories::pipeline_repository::PipelineRepoError;
use crate::services::practice_discuss::thread_dtos;
use crate::services::practice_read_gather::PayloadFailure;
use crate::state::AppState;

/// Every distinct way a discussion request can fail — one observable each.
#[derive(Debug, thiserror::Error)]
pub enum DiscussError {
    #[error("a message needs some text")]
    BlankText,
    #[error("Model '{0}' not available.")]
    UnknownModel(String),
    #[error("practice question {0} not found")]
    QuestionNotFound(Uuid),
    #[error("question {question_id} has reached its limit of {max} model replies")]
    CapReached { question_id: Uuid, max: u32 },
    #[error("the question's context could not be gathered: {0}")]
    Payload(#[from] PayloadFailure),
    #[error("the discussion could not be set up: {0}")]
    Setup(String),
    #[error("the model call failed: {0}")]
    Call(String),
    #[error("{operation} failed for question {question_id}: {source}")]
    Store {
        operation: &'static str,
        question_id: Uuid,
        #[source]
        source: PipelineRepoError,
    },
}

/// Wrap a repository failure with the operation and question it happened on.
pub(crate) fn store(
    operation: &'static str,
    question_id: Uuid,
) -> impl FnOnce(PipelineRepoError) -> DiscussError {
    move |source| DiscussError::Store {
        operation,
        question_id,
        source,
    }
}

/// The whole dock payload for one question.
///
/// # Errors
/// [`DiscussError::QuestionNotFound`], or a named store failure.
pub async fn load_discussion(
    state: &AppState,
    question_id: Uuid,
) -> Result<DiscussionPayload, DiscussError> {
    let question = require_question(state, question_id).await?;
    let settings = state.settings.current();
    let read = &settings.practice_read;

    let scenario = get_scenario(&state.pipeline_pool, question.scenario_id)
        .await
        .map_err(store("get_scenario", question_id))?
        .ok_or(DiscussError::QuestionNotFound(question_id))?;
    let deck = list_deck(&state.pipeline_pool, question.scenario_id)
        .await
        .map_err(store("list_deck", question_id))?;
    let turns = list_thread(&state.pipeline_pool, question_id)
        .await
        .map_err(store("list_thread", question_id))?;
    let used = turns
        .iter()
        .filter(|t| t.role == TurnRole::Model.code())
        .count();

    Ok(DiscussionPayload {
        question_id,
        scenario_code: scenario_code(scenario.code_ordinal),
        position: deck_position(&deck, question_id),
        turns: thread_dtos(&turns, &read.case_timezone),
        models: offered_models(state, question_id).await?,
        default_model: read.discuss_default_model.clone(),
        max_turns: read.discuss_max_turns,
        model_turns: u32::try_from(used).unwrap_or(u32::MAX),
    })
}

/// The question, or a 404-shaped error.
pub(crate) async fn require_question(
    state: &AppState,
    question_id: Uuid,
) -> Result<PracticeQuestionRecord, DiscussError> {
    get_question(&state.pipeline_pool, question_id)
        .await
        .map_err(store("get_question", question_id))?
        .ok_or(DiscussError::QuestionNotFound(question_id))
}

/// The models the picker offers: active catalogue rows the chat provider map holds.
async fn offered_models(
    state: &AppState,
    question_id: Uuid,
) -> Result<Vec<DiscussModelDto>, DiscussError> {
    let rows = list_active_models(&state.pipeline_pool)
        .await
        .map_err(|e| store("list_active_models", question_id)(PipelineRepoError::from(e)))?;
    let mut models: Vec<DiscussModelDto> = rows
        .into_iter()
        .filter(|m| state.chat_providers.contains_key(&m.id))
        .map(|m| DiscussModelDto {
            model_id: m.id,
            display_name: m.display_name,
            billing_class: m.billing_class,
        })
        .collect();
    models.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(models)
}

/// A question's 1-based place in its scenario's visible deck (0 if hidden).
pub fn deck_position(deck: &[PracticeQuestionRecord], question_id: Uuid) -> u32 {
    deck.iter()
        .filter(|q| q.hidden_at.is_none())
        .position(|q| q.id == question_id)
        .map_or(0, |i| u32::try_from(i + 1).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(n: u128, hidden: bool) -> PracticeQuestionRecord {
        PracticeQuestionRecord {
            id: Uuid::from_u128(n),
            scenario_id: Uuid::nil(),
            side: "george".to_string(),
            text: format!("q{n}"),
            tactic: None,
            braid_rows: None,
            source_kind: "manual".to_string(),
            source_ref: None,
            receipt: None,
            watch_for: None,
            stronger: None,
            stronger_lean: None,
            pair_said: None,
            pair_admitted: None,
            sort_order: 1,
            flag_note: None,
            deck_key: None,
            kind: "cross".to_string(),
            follows_key: None,
            source_line: None,
            hidden_at: hidden.then(chrono::Utc::now),
            draft_by: None,
            updated_at: chrono::Utc::now(),
        }
    }

    /// 1-based among VISIBLE questions; a hidden one does not take a number; an
    /// absent (or hidden) question is 0.
    #[test]
    fn deck_position_counts_visible_questions_only() {
        let deck = vec![q(1, false), q(2, true), q(3, false)];
        assert_eq!(deck_position(&deck, Uuid::from_u128(1)), 1);
        assert_eq!(
            deck_position(&deck, Uuid::from_u128(3)),
            2,
            "the hidden q2 is skipped"
        );
        assert_eq!(
            deck_position(&deck, Uuid::from_u128(2)),
            0,
            "a hidden question has no place"
        );
        assert_eq!(
            deck_position(&deck, Uuid::from_u128(9)),
            0,
            "not in the deck"
        );
    }
}
