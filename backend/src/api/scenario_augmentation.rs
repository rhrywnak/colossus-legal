//! HTTP routes for the augmentation panel (task 1.4, v2 §8).
//!
//! - `POST   /cases/:slug/scenarios/:id/human-facts`               → add a C4 fact
//! - `DELETE /cases/:slug/scenarios/:id/human-facts/:fact_id`      → remove one
//! - `PUT    /cases/:slug/scenarios/:id/human-facts/:fact_id`      → rewrite one
//! - `PUT    /cases/:slug/scenarios/:id/talking-points`            → replace C5
//! - `PUT    /cases/:slug/scenarios/:id/talking-points/:position`  → rewrite one
//!
//! The panel's READ lives next door in `scenario_augmentation_read` — Rule 17,
//! and the seam that was always there. The route table below still declares it,
//! because a reader looking for what this family serves should find one list.
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
    domain::human_authored::HumanFactKind,
    dto::scenario_augmentation::{
        AddHumanFactRequest, EditAuthoredLineRequest, SetTalkingPointsRequest,
    },
    error::AppError,
    services::scenario_augmentation::{
        add_human_fact, edit_talking_point, edit_watch_item, remove_human_fact, set_talking_points,
        AugmentationError, NewHumanFact,
    },
    state::AppState,
};

use super::scenario_augmentation_read::get_augmentation_panel;
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
            delete(delete_scenario_human_fact).put(put_watch_item),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id/talking-points",
            put(put_talking_points),
        )
        // Task 2.11 C. Both join THIS module's fence rather than starting a new
        // one: "one guarded write path" means one fence per concern, not zero new
        // handlers — the same reading task 2.11 B1 applied when it put three
        // accusation routes behind one `fence()`.
        .route(
            "/cases/:slug/scenarios/:scenario_id/talking-points/:position",
            put(put_one_talking_point),
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
pub(super) fn augmentation_error_to_app_error(error: AugmentationError) -> AppError {
    match error {
        AugmentationError::EmptyText
        | AugmentationError::DateTypeWithoutDate { .. }
        | AugmentationError::UnknownDateType { .. }
        | AugmentationError::TooManyTalkingPoints { .. } => AppError::BadRequest {
            message: error.to_string(),
            details: json!({ "reason": "invalid_human_content" }),
        },
        // A 404, not a 500 and not a 400: the address was well-formed and the row
        // is simply not there any more — the normal outcome of two people having
        // the page open when one removes a point. The message says exactly that
        // and says what to do about it.
        AugmentationError::NoSuchRow { .. } => AppError::NotFound {
            message: error.to_string(),
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

    let settings = state.settings.current();
    let kept = set_talking_points(
        &state.pipeline_pool,
        id,
        &payload.points,
        &user.username,
        &settings,
    )
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

/// `PUT …/talking-points/:position` — rewrite ONE point, in place.
///
/// ## Why this exists beside the whole-list write
///
/// The list write deletes the response row and re-inserts every item, which
/// re-stamps each row's author and its written-on date with the editor and today.
/// That is right for a reorder and wrong for a typo fix — and the rehearsal page
/// now offers a typo fix per row. Ruling C4b, 2026-08-06.
///
/// The cap is deliberately NOT re-checked: this route changes no row COUNT, and
/// a list already over a lowered cap would otherwise become uneditable, which
/// would strand a human on exactly the words they need to fix.
#[tracing::instrument(skip(state, user, payload), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn put_one_talking_point(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id, position)): Path<(String, String, usize)>,
    Json(payload): Json<EditAuthoredLineRequest>,
) -> Result<StatusCode, AppError> {
    require_edit(&user)?;

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    edit_talking_point(&state.pipeline_pool, id, position, &payload.text)
        .await
        .map_err(augmentation_error_to_app_error)?;

    tracing::info!(
        scenario_id = %id,
        position,
        author = %user.username,
        // The words themselves are NOT logged: they are a human's authored prose
        // and the log is not the place for case content.
        "rewrote one talking point"
    );

    Ok(StatusCode::OK)
}

/// `PUT …/human-facts/:fact_id` — rewrite ONE watch item, in place.
///
/// Scoped to `watch_list` rows by the store's `WHERE kind = …`: a human FACT
/// carries a date and person references this route knows nothing about, and
/// rewriting one through here would drop nothing but would still be an edit made
/// by a surface that cannot show what it is editing.
#[tracing::instrument(skip(state, user, payload), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn put_watch_item(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id, fact_id)): Path<(String, String, String)>,
    Json(payload): Json<EditAuthoredLineRequest>,
) -> Result<StatusCode, AppError> {
    require_edit(&user)?;

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    let fact_uuid = Uuid::parse_str(&fact_id).map_err(|_| AppError::BadRequest {
        message: "fact id must be a valid UUID".to_string(),
        details: json!({ "field": "fact_id" }),
    })?;

    edit_watch_item(&state.pipeline_pool, id, fact_uuid, &payload.text)
        .await
        .map_err(augmentation_error_to_app_error)?;

    tracing::info!(
        %fact_id,
        scenario_id = %id,
        author = %user.username,
        "rewrote one watch item"
    );

    Ok(StatusCode::OK)
}

#[cfg(test)]
#[path = "scenario_augmentation_tests.rs"]
mod tests;
