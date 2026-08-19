//! The sitting's own address: reading one, resuming it, and starting over.
//!
//! Split from [`super::practice`] on 2026-08-19 when that module passed Rule
//! 17's 300-line limit. The seam is the one Section B of the flow task draws:
//! the sibling serves the DECK (what she could be asked) and the ANSWER family
//! serves one question, while these three serve a SITTING as an addressable
//! thing — `…/practice/:scenarioId/session/:sessionId` — which is what makes the
//! browser's Back button and a mid-session reload work.
//!
//! The routes are still declared in one place (`practice::routes`), because a
//! route table split across two files is how a path stops being served by
//! anything.
//!
//! ## Why these paths are NOT case-scoped
//!
//! `session_id` is a server-minted handle the browser only ever learns by having
//! opened the session. The same argument the answer routes make: a case slug in
//! the path would be ceremony rather than a fence, and the fence that matters —
//! that the sitting names its own scenario — is what the payload carries.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table here lives in `colossus_legal_v2`: `&state.pipeline_pool`.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    dto::practice::{OpenSessionsClosed, SittingPayload},
    error::AppError,
    repositories::pipeline_repository::practice::{end_session, session_scenario},
    repositories::pipeline_repository::practice_flow::{
        answered_question_ids, close_open_sessions_except, get_sitting,
    },
    state::AppState,
};

use super::practice::repo_error;

/// One sitting, so `…/practice/:scenarioId/session/:sessionId` can be reloaded.
///
/// ## Why the queue is SERVED and not recomputed
///
/// The order is the drill. Recomputing it on reload would deal a different
/// question than the one she was on the first time the deck changed underneath
/// her — and the deck can change: Chuck edits it. The stored queue is what she
/// was actually dealt, so it is what a reload resumes.
///
/// # Errors
/// 404 when no session carries that id.
pub async fn get_sitting_route(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<SittingPayload>, AppError> {
    let payload = sitting_payload(&state, session_id).await?;
    tracing::info!(
        %session_id,
        who = %payload.who,
        dealt = payload.queue.len(),
        answered = payload.answered.len(),
        ended = payload.ended,
        "served one practice sitting"
    );
    Ok(Json(payload))
}

/// Read one sitting and everything the page needs to re-enter it.
///
/// Split from the two handlers that both need it so neither has to repeat the
/// 404 or the JSON decode, and so the ONE place that decides what "the queue"
/// means for a session opened before flow v1 is a single function.
///
/// # Errors
/// 404 when no session carries that id.
async fn sitting_payload(state: &AppState, session_id: Uuid) -> Result<SittingPayload, AppError> {
    let sitting = get_sitting(&state.pipeline_pool, session_id)
        .await
        .map_err(|e| repo_error("get_sitting", e))?
        .ok_or_else(|| AppError::NotFound {
            message: format!("practice session {session_id} not found"),
        })?;

    // A session opened before flow v1 carries no queue. EMPTY is the honest
    // answer — the page says it cannot resume — and it is deliberately not the
    // whole deck: dealing a queue nobody chose would be the screen inventing her
    // evening. A queue that fails to decode is logged and read the same way,
    // because a half-parsed order is worse than none.
    let queue: Vec<Uuid> = match sitting.queue {
        None => Vec::new(),
        Some(value) => serde_json::from_value(value).unwrap_or_else(|e| {
            tracing::error!(%session_id, error = %e, "practice: the stored queue would not decode");
            Vec::new()
        }),
    };

    Ok(SittingPayload {
        session_id,
        scenario_id: sitting.scenario_id,
        who: sitting.who,
        queue,
        answered: answered_question_ids(&state.pipeline_pool, session_id)
            .await
            .map_err(|e| repo_error("answered_question_ids", e))?,
        ended: sitting.ended_at.is_some(),
    })
}

/// Re-enter the sitting she walked out of, and retire any older open ones.
///
/// ## Domain note: why the others are closed HERE and not on load
///
/// Nothing closed an abandoned sitting before Section B, so a scenario can carry
/// several. Closing them when the deck is merely READ would end sittings she has
/// not been asked about; closing them when she presses Resume is the first
/// moment she has said which one she means. Nothing is deleted either way —
/// each closed sitting keeps its rows and its own Chuck's sheet.
pub async fn post_resume(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<SittingPayload>, AppError> {
    let payload = sitting_payload(&state, session_id).await?;
    let also = close_open_sessions_except(&state.pipeline_pool, payload.scenario_id, session_id)
        .await
        .map_err(|e| repo_error("close_open_sessions_except", e))?;
    tracing::info!(
        %session_id,
        scenario_id = %payload.scenario_id,
        also_closed = also,
        "practice sitting resumed"
    );
    Ok(Json(payload))
}

/// Close the open sitting — and every older one — and return a clean start card.
///
/// Never a delete: the closed sittings keep their answers and each gets a
/// Chuck's sheet of its own. That is what the stored hint beside the control
/// promises, and this is the code that has to keep the promise.
pub async fn post_start_over(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<OpenSessionsClosed>, AppError> {
    let scenario_id = session_scenario(&state.pipeline_pool, session_id)
        .await
        .map_err(|e| repo_error("session_scenario", e))?
        .ok_or_else(|| AppError::NotFound {
            message: format!("practice session {session_id} not found"),
        })?;

    end_session(&state.pipeline_pool, session_id)
        .await
        .map_err(|e| repo_error("end_session", e))?;
    let also = close_open_sessions_except(&state.pipeline_pool, scenario_id, session_id)
        .await
        .map_err(|e| repo_error("close_open_sessions_except", e))?;

    tracing::info!(%session_id, %scenario_id, also_closed = also, "practice sitting started over");
    Ok(Json(OpenSessionsClosed { also_closed: also }))
}
