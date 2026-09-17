//! "Discuss with AI" — two routes over one question's saved thread.
//!
//! CC_TASK_QUESTION_CHAT_v1 §2:
//!
//! - `GET  /practice/questions/:question_id/discussion` → the thread, composed
//! - `POST /practice/questions/:question_id/discussion` → store a message, ask the
//!   model, store its reply, return the fresh thread
//!
//! Thin handlers: the work is `services::practice_discuss_run`. Any signed-in user
//! may read and write (GO ruling 2) — nothing here calls a `require_ai` route, so
//! the witness is never refused a dock she was offered.
//!
//! ## One status per failure
//!
//! 400 blank text or unknown model (exactly as `/ask`) · 404 no such question ·
//! 409 the per-question reply cap is reached · 503 the model could not be set up
//! or reached · 500 a store failure. Each is logged with the question id.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    dto::practice_discussion::{DiscussionPayload, DiscussionRequest},
    error::AppError,
    services::practice_discuss_load::{load_discussion, DiscussError},
    services::practice_discuss_run::send_message,
    state::AppState,
};

/// One question's thread, its model list and its cap.
///
/// # Errors
/// See the module doc.
pub async fn get_discussion(
    user: AuthUser,
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
) -> Result<Json<DiscussionPayload>, AppError> {
    let payload = load_discussion(&state, question_id)
        .await
        .map_err(|e| to_app_error(question_id, &user, e))?;
    tracing::debug!(%question_id, by = %user.username, turns = payload.turns.len(), "served a discussion");
    Ok(Json(payload))
}

/// Send one message to the question's discussion.
///
/// # Errors
/// See the module doc.
pub async fn post_discussion(
    user: AuthUser,
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
    Json(body): Json<DiscussionRequest>,
) -> Result<Json<DiscussionPayload>, AppError> {
    send_message(&state, &user, question_id, &body)
        .await
        .map(Json)
        .map_err(|e| to_app_error(question_id, &user, e))
}

/// Map each failure to its one status, logging it with the question and the user.
pub(super) fn to_app_error(question_id: Uuid, user: &AuthUser, error: DiscussError) -> AppError {
    let message = error.to_string();
    match error {
        DiscussError::BlankText | DiscussError::UnknownModel(_) => {
            tracing::warn!(%question_id, by = %user.username, %message, "practice discuss: refused");
            AppError::BadRequest {
                message,
                details: serde_json::Value::Null,
            }
        }
        DiscussError::QuestionNotFound(_) => AppError::NotFound { message },
        DiscussError::CapReached { max, .. } => {
            tracing::warn!(%question_id, by = %user.username, max, "practice discuss: reply cap reached");
            AppError::Conflict {
                message,
                details: serde_json::json!({ "max_turns": max }),
            }
        }
        DiscussError::Setup(_) | DiscussError::Call(_) => {
            tracing::error!(%question_id, by = %user.username, %message, "practice discuss: model unavailable");
            AppError::ServiceUnavailable { message }
        }
        DiscussError::Payload(_) | DiscussError::Store { .. } => {
            tracing::error!(%question_id, by = %user.username, %message, "practice discuss: failed");
            AppError::Internal {
                message: "the discussion could not be completed".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    fn user() -> AuthUser {
        AuthUser {
            username: "docmarie".to_string(),
            email: String::new(),
            display_name: "Marie".to_string(),
            groups: Vec::new(),
        }
    }

    fn status(error: DiscussError) -> u16 {
        to_app_error(Uuid::nil(), &user(), error)
            .into_response()
            .status()
            .as_u16()
    }

    /// Every failure has its own status — never collapsed into one 500.
    #[test]
    fn each_failure_maps_to_its_own_status() {
        assert_eq!(status(DiscussError::BlankText), 400);
        assert_eq!(status(DiscussError::UnknownModel("gpt".to_string())), 400);
        assert_eq!(status(DiscussError::QuestionNotFound(Uuid::nil())), 404);
        assert_eq!(
            status(DiscussError::CapReached {
                question_id: Uuid::nil(),
                max: 40
            }),
            409
        );
        assert_eq!(status(DiscussError::Setup("no prompt".to_string())), 503);
        assert_eq!(status(DiscussError::Call("timeout".to_string())), 503);
        assert_eq!(
            status(DiscussError::Payload(
                crate::services::practice_read_gather::PayloadFailure::TacticUnnamed { card: 5 }
            )),
            500
        );
        assert_eq!(
            status(DiscussError::Store {
                operation: "list_thread",
                question_id: Uuid::nil(),
                source: crate::repositories::pipeline_repository::PipelineRepoError::from(
                    sqlx::Error::RowNotFound
                ),
            }),
            500
        );
    }

    /// The unknown-model message matches `/ask`'s wording, naming the id.
    #[test]
    fn unknown_model_names_the_id() {
        assert_eq!(
            DiscussError::UnknownModel("gpt-9".to_string()).to_string(),
            "Model 'gpt-9' not available."
        );
    }
}
