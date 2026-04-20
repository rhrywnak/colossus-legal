//! Admin CRUD endpoints for the `llm_models` table.
//!
//! Design: DOC_PROCESSING_CONFIG_DESIGN_v2.md Section 3.4.1.

use axum::{
    extract::{Path as AxumPath, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::pipeline_repository::models::{
    self, InsertModelInput, LlmModelRecord, UpdateModelInput,
};
use crate::state::AppState;

use super::shared::profiles_referencing;

/// Providers the admin API accepts for a model's `provider` column.
/// Matches the dispatch in `backend/src/pipeline/providers.rs`; additions
/// here must be mirrored there (and vice-versa).
const ALLOWED_PROVIDERS: &[&str] = &["anthropic", "vllm", "openai"];

/// Postgres SQLSTATE for unique-constraint violations.
const UNIQUE_VIOLATION: &str = "23505";

#[derive(Debug, Serialize)]
pub struct ModelsResponse {
    pub models: Vec<LlmModelRecord>,
}

/// Body of POST /models — create a new model.
///
/// Mirrors `InsertModelInput` with an identical shape; defined as a
/// separate HTTP-layer DTO so the API contract can evolve independently
/// of the repository layer.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateModelInput {
    pub id: String,
    pub display_name: String,
    pub provider: String,
    #[serde(default)]
    pub api_endpoint: Option<String>,
    #[serde(default)]
    pub max_context_tokens: Option<i32>,
    #[serde(default)]
    pub max_output_tokens: Option<i32>,
    #[serde(default)]
    pub cost_per_input_token: Option<f64>,
    #[serde(default)]
    pub cost_per_output_token: Option<f64>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// GET /api/admin/pipeline/models — list every model (active and inactive).
///
/// Admin UIs need the full set so operators can re-activate deactivated
/// models. The runtime extraction path uses `list_active_models` instead.
pub async fn list_models(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ModelsResponse>, AppError> {
    require_admin(&user)?;

    let rows = models::list_all_models(&state.pipeline_pool)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to list models: {e}"),
        })?;

    Ok(Json(ModelsResponse { models: rows }))
}

/// POST /api/admin/pipeline/models — create a new model row.
///
/// Validates the id is non-empty and the provider is one of
/// [`ALLOWED_PROVIDERS`]. Returns `409 Conflict` if the id already exists.
pub async fn create_model(
    user: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CreateModelInput>,
) -> Result<Json<LlmModelRecord>, AppError> {
    require_admin(&user)?;

    let id = input.id.trim();
    if id.is_empty() {
        return Err(AppError::BadRequest {
            message: "Model id must not be empty".into(),
            details: serde_json::json!({"field": "id"}),
        });
    }
    if !ALLOWED_PROVIDERS.contains(&input.provider.as_str()) {
        return Err(AppError::BadRequest {
            message: format!(
                "Unknown provider '{}'; expected one of: {}",
                input.provider,
                ALLOWED_PROVIDERS.join(", ")
            ),
            details: serde_json::json!({"field": "provider"}),
        });
    }

    let repo_input = InsertModelInput {
        id: id.to_string(),
        display_name: input.display_name,
        provider: input.provider,
        api_endpoint: input.api_endpoint,
        max_context_tokens: input.max_context_tokens,
        max_output_tokens: input.max_output_tokens,
        cost_per_input_token: input.cost_per_input_token,
        cost_per_output_token: input.cost_per_output_token,
        notes: input.notes,
    };

    match models::insert_model(&state.pipeline_pool, &repo_input).await {
        Ok(record) => Ok(Json(record)),
        Err(e) => Err(map_insert_error(e, id)),
    }
}

/// PUT /api/admin/pipeline/models/:id — patch a model's fields.
///
/// Any field omitted from the request body is left unchanged (`COALESCE`
/// in the repository). Returns `404 Not Found` if the id does not exist.
pub async fn update_model(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(model_id): AxumPath<String>,
    Json(input): Json<UpdateModelInput>,
) -> Result<Json<LlmModelRecord>, AppError> {
    require_admin(&user)?;

    if let Some(provider) = input.provider.as_deref() {
        if !ALLOWED_PROVIDERS.contains(&provider) {
            return Err(AppError::BadRequest {
                message: format!(
                    "Unknown provider '{provider}'; expected one of: {}",
                    ALLOWED_PROVIDERS.join(", ")
                ),
                details: serde_json::json!({"field": "provider"}),
            });
        }
    }

    let updated = models::update_model(&state.pipeline_pool, &model_id, &input)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update model '{model_id}': {e}"),
        })?;

    updated.map(Json).ok_or_else(|| AppError::NotFound {
        message: format!("Model '{model_id}' not found"),
    })
}

/// DELETE /api/admin/pipeline/models/:id — delete a model row.
///
/// Refuses the delete (`409 Conflict`) if any profile YAML under
/// `processing_profile_dir` references this model's id. The check is
/// filesystem-based because profiles live on disk, not in the database.
pub async fn delete_model(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(model_id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&user)?;

    let referencing = profiles_referencing(&state.config.processing_profile_dir, &model_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to scan profile directory: {e}"),
        })?;

    if !referencing.is_empty() {
        return Err(AppError::Conflict {
            message: format!(
                "Model '{model_id}' is referenced by {} profile(s)",
                referencing.len()
            ),
            details: serde_json::json!({"referenced_by": referencing}),
        });
    }

    let deleted = models::delete_model(&state.pipeline_pool, &model_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to delete model '{model_id}': {e}"),
        })?;

    if !deleted {
        return Err(AppError::NotFound {
            message: format!("Model '{model_id}' not found"),
        });
    }

    Ok(Json(serde_json::json!({"deleted": model_id})))
}

/// PUT /api/admin/pipeline/models/:id/toggle — flip `is_active`.
///
/// Returns the updated row. `404 Not Found` if the id does not exist.
pub async fn toggle_model(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(model_id): AxumPath<String>,
) -> Result<Json<LlmModelRecord>, AppError> {
    require_admin(&user)?;

    let updated = models::toggle_model_active(&state.pipeline_pool, &model_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to toggle model '{model_id}': {e}"),
        })?;

    updated.map(Json).ok_or_else(|| AppError::NotFound {
        message: format!("Model '{model_id}' not found"),
    })
}

/// Map an `sqlx::Error` from `insert_model` into the correct HTTP error.
///
/// A `23505` unique-violation surfaces as `409 Conflict`; anything else
/// is an unexpected internal failure.
fn map_insert_error(e: sqlx::Error, id: &str) -> AppError {
    if let sqlx::Error::Database(db_err) = &e {
        if db_err.code().as_deref() == Some(UNIQUE_VIOLATION) {
            return AppError::Conflict {
                message: format!("Model '{id}' already exists"),
                details: serde_json::json!({"id": id}),
            };
        }
    }
    AppError::Internal {
        message: format!("Failed to insert model: {e}"),
    }
}
