use axum::{extract::Path, extract::State, http::StatusCode, Json};
use serde_json::json;

use crate::{
    auth::{AuthUser, require_edit},
    dto::ClaimDto,
    dto::{ClaimCreateRequest, ClaimUpdateRequest, MotionClaimsResponse},
    error::AppError,
    models::claim::Claim,
    repositories::claim_repository::ClaimRepository,
    repositories::MotionClaimRepository,
    state::AppState,
};

const ALLOWED_STATUSES: &[&str] = &["open", "closed", "refuted", "pending"];

fn validate_title(title: &str) -> Result<(), AppError> {
    if title.trim().is_empty() {
        return Err(AppError::BadRequest {
            message: "title must not be empty".to_string(),
            details: json!({ "field": "title" }),
        });
    }
    Ok(())
}

fn validate_status(status: &str) -> Result<(), AppError> {
    if !ALLOWED_STATUSES.contains(&status) {
        return Err(AppError::BadRequest {
            message: "status must be one of: open, closed, refuted, pending".to_string(),
            details: json!({ "field": "status" }),
        });
    }
    Ok(())
}

fn to_dto(claim: Claim) -> ClaimDto {
    ClaimDto {
        id: claim.id,
        title: claim.title,
        description: claim.description,
        status: claim.status,
    }
}

pub async fn list_claims(
    user: Option<AuthUser>,
    State(state): State<AppState>,
) -> Result<Json<Vec<ClaimDto>>, AppError> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /claims", u.username);
    }
    let repo = ClaimRepository::new(state.graph.clone());
    let claims = repo.list_claims().await.map_err(|_| AppError::Internal {
        message: "failed to list claims".to_string(),
    })?;

    let dtos = claims.into_iter().map(to_dto).collect();

    Ok(Json(dtos))
}

pub async fn get_claim(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ClaimDto>, AppError> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /claims/{}", u.username, id);
    }
    let repo = ClaimRepository::new(state.graph.clone());
    let claim = repo.get_claim_by_id(&id).await.map_err(|err| match err {
        crate::repositories::claim_repository::ClaimRepositoryError::NotFound => {
            AppError::NotFound {
                message: "claim not found".to_string(),
            }
        }
        _ => AppError::Internal {
            message: "failed to fetch claim".to_string(),
        },
    })?;

    Ok(Json(to_dto(claim)))
}

pub async fn create_claim(
    user: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<ClaimCreateRequest>,
) -> Result<(axum::http::StatusCode, Json<ClaimDto>), AppError> {
    require_edit(&user)?;
    tracing::info!("{} POST /claims", user.username);
    validate_title(&payload.title)?;
    validate_status(&payload.status)?;

    let repo = ClaimRepository::new(state.graph.clone());
    let claim = repo
        .create_claim(
            &payload.title,
            payload.description.as_deref(),
            &payload.status,
        )
        .await
        .map_err(|err| match err {
            crate::repositories::claim_repository::ClaimRepositoryError::CreationFailed => {
                AppError::Internal {
                    message: "failed to create claim".to_string(),
                }
            }
            _ => AppError::Internal {
                message: "failed to create claim".to_string(),
            },
        })?;

    Ok((axum::http::StatusCode::CREATED, Json(to_dto(claim))))
}

pub async fn update_claim(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ClaimUpdateRequest>,
) -> Result<Json<ClaimDto>, AppError> {
    require_edit(&user)?;
    tracing::info!("{} PUT /claims/{}", user.username, id);
    if let Some(title) = payload.title.as_deref() {
        validate_title(title)?;
    }
    if let Some(status) = payload.status.as_deref() {
        validate_status(status)?;
    }

    let repo = ClaimRepository::new(state.graph.clone());
    let updated = repo
        .update_claim(
            &id,
            payload.title.as_deref(),
            payload.description.as_deref(),
            payload.status.as_deref(),
        )
        .await
        .map_err(|err| match err {
            crate::repositories::claim_repository::ClaimRepositoryError::NotFound => {
                AppError::NotFound {
                    message: "claim not found".to_string(),
                }
            }
            _ => AppError::Internal {
                message: "failed to update claim".to_string(),
            },
        })?;

    Ok(Json(to_dto(updated)))
}

/// List all MotionClaim nodes with their relationships
/// (PROVES -> Allegation, RELIES_ON -> Evidence, APPEARS_IN -> Document)
pub async fn list_motion_claims(
    user: Option<AuthUser>,
    State(state): State<AppState>,
) -> Result<Json<MotionClaimsResponse>, StatusCode> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /motion-claims", u.username);
    }
    let repo = MotionClaimRepository::new(state.graph.clone());
    match repo.list_motion_claims().await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::error!("Failed to fetch motion claims: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
