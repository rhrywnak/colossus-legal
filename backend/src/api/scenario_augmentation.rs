//! HTTP routes for the augmentation panel (task 1.4, v2 §8).
//!
//! - `GET    /cases/:slug/scenarios/:id/augmentation`            → the panel
//! - `POST   /cases/:slug/scenarios/:id/human-facts`             → add a C4 fact
//! - `DELETE /cases/:slug/scenarios/:id/human-facts/:fact_id`    → remove one
//! - `PUT    /cases/:slug/scenarios/:id/talking-points`          → replace C5
//!
//! C1 identity is edited through the existing `PUT /scenarios/:id`, which task
//! 1.4 extended with `theme_statement` and `motivation` — a second route for two
//! columns on a row that already has an update path would be a second way to do
//! one thing.
//!
//! ## The §8 invariants at this layer
//!
//! No handler here calls gather, and no scan path calls the augmentation service.
//! Both directions are asserted by source scans rather than trusted:
//! `augmentation_never_gathers` (service) and
//! `scan_and_merge_paths_write_only_their_own_tables` (repository).
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table here lives in `colossus_legal_v2`, so all reads and writes use
//! `&state.pipeline_pool`, NOT `state.pg_pool`.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::NaiveDate;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{require_edit, AuthUser},
    domain::human_authored::{authored_tag, talking_points_cap, DateType, HumanFactKind},
    domain::scenario_code::scenario_code,
    dto::scenario_augmentation::{
        AddHumanFactRequest, AugmentationPanelDto, HumanFactDto, ScenarioIdentityDto,
        SetTalkingPointsRequest, TalkingPointDto,
    },
    error::AppError,
    repositories::pipeline_repository::{get_scenario, ScenarioHumanFactRecord, ScenarioRecord},
    services::scenario_augmentation::{
        add_human_fact, human_facts, remove_human_fact, set_talking_points, talking_points,
        AugmentationError, NewHumanFact,
    },
    state::AppState,
};

use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};

/// This module's routes.
///
/// Declared here rather than in `api::mod`'s central table: the table had grown
/// past the module-size limit, and routes read better beside the handlers they
/// name — a reader looking at `put_talking_points` can see its path without
/// opening another file.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/cases/:slug/scenarios/:scenario_id/augmentation",
            get(get_augmentation_panel),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id/human-facts",
            post(add_scenario_human_fact),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id/human-facts/:fact_id",
            delete(delete_scenario_human_fact),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id/talking-points",
            put(put_talking_points),
        )
}

/// Map an [`AugmentationError`] onto its HTTP status.
///
/// ## Why these messages reach the client verbatim
///
/// Every refusal here is about what the HUMAN just typed — an empty note, a
/// fourth talking point, a date qualifier with no date. They are the only one who
/// can act on any of it, so a generic "failed to save" would leave them guessing.
/// The two write-failure cases stay opaque, because those are about the server.
fn augmentation_error_to_app_error(error: AugmentationError) -> AppError {
    match error {
        AugmentationError::EmptyText
        | AugmentationError::DateTypeWithoutDate { .. }
        | AugmentationError::UnknownDateType { .. }
        | AugmentationError::TooManyTalkingPoints { .. } => AppError::BadRequest {
            message: error.to_string(),
            details: json!({ "reason": "invalid_human_content" }),
        },
        AugmentationError::Write { .. } => {
            tracing::error!(error = %error, "failed to store human-authored content");
            AppError::Internal {
                message: "failed to save".to_string(),
            }
        }
        AugmentationError::Read { .. } => {
            tracing::error!(error = %error, "failed to read human-authored content");
            AppError::Internal {
                message: "failed to load".to_string(),
            }
        }
    }
}

/// Compose one human fact for display.
///
/// The date label and the authored tag are built HERE, not in the browser: the
/// qualifier ("Around …") is a claim about precision, and the tag is this
/// content's only provenance. Both are case vocabulary, and the language law puts
/// every such string on this side of the wire.
fn to_fact_dto(record: &ScenarioHumanFactRecord) -> HumanFactDto {
    let date_label = record.occurred_on.map(|date| {
        let rendered = date.to_string();
        match record.date_type.as_deref().map(DateType::try_from) {
            // A stored type this build cannot read renders as the bare date
            // rather than guessing a qualifier — understating precision is the
            // safe direction.
            Some(Ok(kind)) => kind.describe(&rendered),
            _ => rendered,
        }
    });

    HumanFactDto {
        id: record.id.to_string(),
        text: record.text.clone(),
        date_label,
        person_refs: record.person_refs.clone().unwrap_or_default(),
        // Always false before task B0: these are names a human typed, not
        // resolved entities, and the panel says so.
        person_refs_are_linked: false,
        authored_tag: authored_tag(&record.authored_by),
        // Equal stamps mean untouched since it was written (see the insert).
        edited: record.updated_at != record.created_at,
    }
}

/// Compose the C1 identity block.
fn to_identity_dto(record: &ScenarioRecord) -> ScenarioIdentityDto {
    ScenarioIdentityDto {
        code: scenario_code(record.code_ordinal),
        name: record.name.clone(),
        direction: record.direction.clone(),
        theme_statement: record.theme_statement.clone(),
        motivation: record.motivation.clone(),
        // Read out of the opaque definition body. A definition that is `{}` or a
        // retired shape simply yields `None` — the panel shows no attack line
        // rather than failing, because C1 editing must work on a half-authored
        // scenario.
        attack_text: record
            .definition
            .get("attack_text")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    }
}

/// `GET …/augmentation` — the whole panel in one read.
///
/// Open read (`Option<AuthUser>`), matching the sibling scenario reads.
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn get_augmentation_panel(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<AugmentationPanelDto>, AppError> {
    if let Some(ref u) = user {
        tracing::info!("{} GET augmentation panel for {}", u.username, scenario_id);
    }

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    let record = get_scenario(&state.pipeline_pool, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, scenario_id = %id, "failed to read scenario for panel");
            AppError::Internal {
                message: "failed to load the scenario".to_string(),
            }
        })?
        .ok_or_else(|| AppError::NotFound {
            message: "scenario not found".to_string(),
        })?;

    let facts = human_facts(&state.pipeline_pool, id)
        .await
        .map_err(augmentation_error_to_app_error)?;

    let points = talking_points(&state.pipeline_pool, id)
        .await
        .map_err(augmentation_error_to_app_error)?;

    Ok(Json(AugmentationPanelDto {
        identity: to_identity_dto(&record),
        human_facts: facts
            .iter()
            .filter(|f| f.kind == HumanFactKind::Fact.code())
            .map(to_fact_dto)
            .collect(),
        watch_list: facts
            .iter()
            .filter(|f| f.kind == HumanFactKind::WatchList.code())
            .map(to_fact_dto)
            .collect(),
        talking_points: points
            .into_iter()
            .map(|item| TalkingPointDto {
                text: item.text,
                position: item.item_index,
                authored_tag: item.authored_by.as_deref().map(authored_tag),
            })
            .collect(),
        talking_points_cap: talking_points_cap(),
    }))
}

/// `POST …/human-facts` — add one C4 fact.
#[tracing::instrument(skip(state, user, payload), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn add_scenario_human_fact(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
    Json(payload): Json<AddHumanFactRequest>,
) -> Result<StatusCode, AppError> {
    require_edit(&user)?;

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    // Parse the date at the HTTP boundary: a malformed date is a client error
    // with a precise message, not a database rejection later.
    let occurred_on =
        match payload.occurred_on.as_deref() {
            Some(raw) => Some(NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(|_| {
                AppError::BadRequest {
                    message: format!("'{raw}' is not a date (expected YYYY-MM-DD)"),
                    details: json!({ "field": "occurred_on" }),
                }
            })?),
            None => None,
        };

    // Parse the kind at the boundary too, for the same reason as the date: an
    // unreadable token becomes a named 400 here rather than a row this build
    // cannot classify later.
    let kind = match payload.kind.as_deref() {
        Some(raw) => HumanFactKind::try_from(raw).map_err(|_| AppError::BadRequest {
            message: format!(
                "'{raw}' is not a note kind this build understands (fact, watch_list)"
            ),
            details: json!({ "field": "kind" }),
        })?,
        None => HumanFactKind::Fact,
    };

    let fact_id = add_human_fact(
        &state.pipeline_pool,
        &NewHumanFact {
            scenario_id: id,
            text: &payload.text,
            kind,
            occurred_on,
            date_type: payload.date_type.as_deref(),
            person_refs: &payload.person_refs,
            authored_by: &user.username,
        },
    )
    .await
    .map_err(augmentation_error_to_app_error)?;

    tracing::info!(
        %fact_id,
        scenario_id = %id,
        author = %user.username,
        kind = kind.code(),
        "added a human note"
    );

    Ok(StatusCode::CREATED)
}

/// `DELETE …/human-facts/:fact_id` — remove one C4 fact.
///
/// A delete that removes nothing is a 404, distinct from a successful 204: the
/// two are different states and collapsing them would hide a wrong id.
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn delete_scenario_human_fact(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id, fact_id)): Path<(String, String, String)>,
) -> Result<StatusCode, AppError> {
    require_edit(&user)?;

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    let fact_uuid = Uuid::parse_str(&fact_id).map_err(|_| AppError::BadRequest {
        message: "fact id must be a valid UUID".to_string(),
        details: json!({ "field": "fact_id" }),
    })?;

    let removed = remove_human_fact(&state.pipeline_pool, id, fact_uuid)
        .await
        .map_err(augmentation_error_to_app_error)?;

    if !removed {
        return Err(AppError::NotFound {
            message: "no such human fact on this scenario".to_string(),
        });
    }

    tracing::info!(%fact_id, scenario_id = %id, author = %user.username, "removed a human fact");
    Ok(StatusCode::NO_CONTENT)
}

/// `PUT …/talking-points` — replace the scenario's C5 list.
#[tracing::instrument(skip(state, user, payload), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn put_talking_points(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
    Json(payload): Json<SetTalkingPointsRequest>,
) -> Result<StatusCode, AppError> {
    require_edit(&user)?;

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    let kept = set_talking_points(&state.pipeline_pool, id, &payload.points, &user.username)
        .await
        .map_err(augmentation_error_to_app_error)?;

    tracing::info!(
        scenario_id = %id,
        points = kept.len(),
        author = %user.username,
        "replaced the talking points"
    );

    Ok(StatusCode::OK)
}

#[cfg(test)]
#[path = "scenario_augmentation_tests.rs"]
mod tests;
