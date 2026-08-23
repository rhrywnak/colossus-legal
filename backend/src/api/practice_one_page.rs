//! The one-page surface's two endpoints: the answers sheet, and the sitting
//! nobody sees.
//!
//! Split from [`super::practice`] the day they were written, when the second of
//! them carried that module to 320 lines (Rule 17). The seam is the one the
//! task draws rather than an arbitrary halving: everything here exists because
//! `CC_TASK_PRACTICE_ONE_PAGE` retired the sitting apparatus from the
//! interface, and both will be read together by whoever next asks how that
//! retirement was actually done.
//!
//! The routes stay declared in `practice::routes` with all the others, because
//! a route table split across files is how a path stops being served by
//! anything.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::dto::practice::{AnswerVersionDto, QuestionAnswersPayload, StartSessionResponse};
use crate::error::AppError;
use crate::repositories::pipeline_repository::{
    practice::{start_session, NewSitting},
    practice_flow::{answer_versions, current_answers, open_session_for_answers},
};
use crate::services::practice_notes::attribution;
use crate::state::AppState;

/// The `who` a sitting opened to hold answers records.
///
/// // CONST: a schema discriminator, not a setting. `practice_sessions.who` has
/// a CHECK constraining it to three values, so changing this needs a migration
/// that back-fills existing rows — it cannot be made configurable without
/// simultaneous database work.
///
/// `mixed` because there is no side to choose: the *Who's asking?* selector is
/// retired, and a sitting claiming one side would misdescribe a page that shows
/// both. It already means "all of them, unlimited" everywhere else.
const SESSION_WHO_MIXED: &str = "mixed";

use super::practice::repo_error;
use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};

/// Every current answer in one scenario — the printed answers sheet's payload.
///
/// ## Why a second endpoint and not a field on the deck payload
///
/// The deck payload is fetched on EVERY load of the practice page, and this
/// carries every answer's full prose. Riding it along would make Marie wait on
/// text her screen never shows, on the one surface whose whole promise is that
/// a witness never waits on a network. Chuck asks for this once, deliberately,
/// by opening a print tab.
///
/// ## Domain note: scenario-wide, like the line on the deck row
///
/// Not scoped to the requester. Chuck opens this to read MARIE's answers, and a
/// user-scoped read would hand him blank paper. See `current_answers`.
///
/// # Errors
/// 404 for a scenario outside this case; 500 (logged, operation named) for a
/// read that fails.
pub async fn get_practice_answers(
    // ## ⚑ Present for AUTHENTICATION, not for scoping
    //
    // The query below is deliberately scenario-wide — Chuck reads MARIE's
    // answers, so the data is not filtered by who asked. That is a statement
    // about SCOPING and says nothing about whether the caller may be here at
    // all. This extractor is the backend's own 401: with `AUTH_MODE=Required` a
    // request carrying no Authentik headers is refused before this body runs,
    // and without it the handler would trust Traefik alone. Every sibling read
    // in this module takes it for exactly that reason.
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<crate::dto::practice::PracticeAnswersPayload>, AppError> {
    let scenario_id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, scenario_id, &slug).await?;

    let settings = state.settings.current();
    let current = current_answers(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("current_answers", e))?;

    let answers = current
        .into_iter()
        .map(|record| crate::dto::practice::PracticeAnswerDto {
            question_id: record.question_id,
            text: record.answer_text,
            // Composed here, like every other sentence on this surface: the
            // client holds no templates and no date format.
            answered_on: crate::services::practice_page::answered_on_line(
                &settings,
                record.answered_at,
            ),
        })
        .collect::<Vec<_>>();

    // WHO asked is worth a field even though it does not change WHAT is served:
    // this is Marie's writing leaving the building on paper, and a log that
    // cannot say who printed it cannot answer the question afterwards.
    tracing::info!(
        slug = %slug,
        %scenario_id,
        by = %user.username,
        answers = answers.len(),
        "served the practice answers"
    );
    Ok(Json(crate::dto::practice::PracticeAnswersPayload {
        answers,
    }))
}

/// The session id an answer from the question page will belong to.
///
/// ## ⚑ "No sittings" is true of the interface and FALSE of the database
///
/// The one-page work retired Start, the counts, the sides, resume and end from
/// the screen. It did not — and could not — retire the row: an answer's
/// `session_id` is `NOT NULL REFERENCES practice_sessions(id)`, so every answer
/// belongs to a sitting whether or not anybody is shown one. Roman ruled on
/// 2026-08-23 to keep the row and hide the concept, and this endpoint is the
/// whole of the hiding.
///
/// Reuses the newest unended sitting for this scenario and user, or opens one.
/// `who` is `mixed` because there is no side to choose any more; the queue is
/// empty because nothing deals questions.
///
/// # Errors
/// 404 for a scenario outside this case; 500 (logged, operation named) on a
/// read or write that fails.
pub async fn post_answer_session(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<StartSessionResponse>, AppError> {
    let scenario_id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, scenario_id, &slug).await?;

    let (user_id, user_name) = attribution(&user);
    if let Some(existing) = open_session_for_answers(&state.pipeline_pool, scenario_id, &user_id)
        .await
        .map_err(|e| repo_error("open_session_for_answers", e))?
    {
        tracing::debug!(%scenario_id, session_id = %existing, "reusing the open sitting for answers");
        return Ok(Json(StartSessionResponse {
            session_id: existing,
        }));
    }

    let session_id = start_session(
        &state.pipeline_pool,
        scenario_id,
        // No side to choose: the *Who\'s asking?* selector is retired, and a
        // sitting that claimed one side would misdescribe a page that shows both.
        SESSION_WHO_MIXED,
        &NewSitting {
            user_id: &user_id,
            user_name: &user_name,
            count: None,
            // An EMPTY json array, not null: the column is `NOT NULL`, and an
            // empty queue is the honest record of a sitting that deals nothing.
            queue: &serde_json::Value::Array(Vec::new()),
            skipped_today: &serde_json::Value::Array(Vec::new()),
        },
    )
    .await
    .map_err(|e| repo_error("start_session", e))?;

    tracing::info!(%scenario_id, %session_id, by = %user_id, "opened a sitting to hold answers");
    Ok(Json(StartSessionResponse { session_id }))
}

/// One question's answer history — what stands now, and what came before.
///
/// ## Domain note: addressed by the QUESTION's own id
///
/// Not case- and scenario-scoped like its neighbours, for the reason every
/// `/practice/questions/:id/…` route in this API is not: the id is server-minted
/// and unguessable, and the alternative — threading a case and a scenario the
/// caller already proved once — buys nothing a reader of the route can see.
///
/// # Errors
/// 500 (logged, operation named) for a read that fails.
pub async fn get_question_answers(
    user: AuthUser,
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
) -> Result<Json<QuestionAnswersPayload>, AppError> {
    let settings = state.settings.current();
    let mut versions = answer_versions(&state.pipeline_pool, question_id)
        .await
        .map_err(|e| repo_error("answer_versions", e))?
        .into_iter()
        .map(|record| AnswerVersionDto {
            answer_id: record.answer_id,
            text: record.answer_text,
            answered_on: crate::services::practice_page::answered_on_line(
                &settings,
                record.answered_at,
            ),
        })
        .collect::<Vec<_>>();

    // Newest first from the query, so the CURRENT answer is the head. Draining
    // it out is what makes the two fields mean different things — see the DTO.
    let current = if versions.is_empty() {
        None
    } else {
        Some(versions.remove(0))
    };

    tracing::debug!(
        %question_id,
        by = %user.username,
        earlier = versions.len(),
        "served one question's answers"
    );
    Ok(Json(QuestionAnswersPayload {
        current,
        earlier: versions,
    }))
}
