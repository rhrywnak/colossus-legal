//! GET /api/chat/models — catalog of chat-selectable LLMs.
//!
//! Backs the frontend model dropdown (Part 3/3). Reads every active row
//! from `llm_models`, marks the server's configured default, and returns
//! the list verbatim — the handler does NOT filter against
//! `chat_providers`, because the catalog is a DB-level truth and the map
//! only exists when `ANTHROPIC_API_KEY` is configured.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::api::embed::ErrorResponse;
use crate::auth::{require_ai, AuthUser};
use crate::repositories::pipeline_repository::models;
use crate::state::AppState;

/// A single entry in the chat-models response.
#[derive(Debug, Serialize)]
pub struct ChatModelEntry {
    pub model_id: String,
    pub display_name: String,
    /// True when this row's id equals `AppState::default_chat_model`.
    pub is_default: bool,
}

/// Response body for `GET /api/chat/models`.
#[derive(Debug, Serialize)]
pub struct ChatModelsResponse {
    pub models: Vec<ChatModelEntry>,
    pub default_model: String,
}

type ApiError = (axum::http::StatusCode, Json<ErrorResponse>);

/// `GET /api/chat/models` handler. Requires AI-role auth so it matches
/// `/ask`'s access rules — the catalog exposes which models the user can
/// actually select for synthesis.
pub async fn list_chat_models(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ChatModelsResponse>, ApiError> {
    require_ai(&user).map_err(|e| {
        (
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: e.message }),
        )
    })?;

    let rows = models::list_active_models(&state.pipeline_pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list active llm_models");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("DB error: {e}"),
                }),
            )
        })?;

    let default_model = state.default_chat_model.clone();
    let entries = rows
        .into_iter()
        .map(|m| ChatModelEntry {
            is_default: m.id == default_model,
            model_id: m.id,
            display_name: m.display_name,
        })
        .collect();

    Ok(Json(ChatModelsResponse {
        models: entries,
        default_model,
    }))
}
