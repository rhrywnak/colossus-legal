//! HTTP routes for rehearsal mode and the ready gate (task 1.5, v2 §5/§6/§10).
//!
//! - `POST /cases/:slug/scenarios/:id/ready` → declare ready / take back out
//! - `GET  /cases/:slug/rehearsal`           → every ready scenario, for Marie
//!
//! ## Why the ready toggle is a route of its own
//!
//! `PUT /scenarios/:id` edits a scenario's fields and records no actor. Readiness
//! is not a field edit — it is the act that puts a scenario in front of a witness,
//! and §5/§6 make both directions human acts with a name attached. As of this task
//! that PUT REFUSES `status` outright, so this is the only path, and every
//! transition has an actor.
//!
//! ## Why rehearsal is read at the CASE level
//!
//! §10 consumes scenario-by-scenario in short sessions, but the list of what is
//! rehearsable is the case's. One read gives the page everything it needs, so
//! moving between scenarios with the keyboard is instant and needs no network —
//! which is the difference between a rehearsal surface and a data table.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table here lives in `colossus_legal_v2`: `&state.pipeline_pool`.

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    auth::{require_edit, AuthUser},
    domain::scenario_code::scenario_code,
    dto::rehearsal::RehearsalPayload,
    error::AppError,
    repositories::pipeline_repository::get_scenario,
    services::scenario_readiness::{
        rehearsal_payload, set_readiness, ReadinessError, READY_STATUS,
    },
    state::AppState,
};

use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};

/// This module's routes (declared beside their handlers — see
/// `scenario_augmentation::routes` for why the central table stopped growing).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/cases/:slug/scenarios/:scenario_id/ready",
            post(set_scenario_ready),
        )
        .route("/cases/:slug/rehearsal", get(get_rehearsal))
}

/// Request body for the ready toggle.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetReadyRequest {
    /// `true` declares it ready; `false` takes it back out of rehearsal.
    ///
    /// Explicit rather than a bare toggle: a toggle acts on state the client
    /// believed was current, so two people on the page at once can each press
    /// "ready" and end with it drafted. Stating the target makes the request
    /// mean the same thing whenever it arrives.
    pub ready: bool,
}

/// What changed, in words the human can read back.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadyChangeDto {
    pub status: String,
    /// True when the scenario is now visible in rehearsal mode.
    pub in_rehearsal: bool,
    /// The plain confirmation, composed here — §10's language law.
    pub message: String,
}

/// `POST …/ready` — declare a scenario ready, or take it back out.
///
/// ## Domain note: the demotion message says what did NOT happen
///
/// "Removed from rehearsal — nothing else changed" is not reassurance padding.
/// Demoting is the act a human hesitates over the night before trial, and the
/// hesitation is *does this throw away the work*. It does not: the evidence cut,
/// the rulings, the talking points and the human facts are all untouched. Saying
/// so is what makes the toggle usable in both directions.
#[tracing::instrument(skip(state, user, payload), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn set_scenario_ready(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
    Json(payload): Json<SetReadyRequest>,
) -> Result<Json<ReadyChangeDto>, AppError> {
    require_edit(&user)?;

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    let record = get_scenario(&state.pipeline_pool, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %id, slug = %slug, "failed to read the scenario before a readiness change");
            AppError::Internal {
                message: "failed to load the scenario".to_string(),
            }
        })?
        .ok_or_else(|| AppError::NotFound {
            message: "no such scenario in this case".to_string(),
        })?;

    let code = scenario_code(record.code_ordinal);
    let status = set_readiness(&state.pipeline_pool, &record, payload.ready, &user.username)
        .await
        .map_err(readiness_error_to_app_error)?;

    tracing::info!(
        %id,
        scenario = %code,
        from = %record.status,
        to = %status,
        actor = %user.username,
        "recorded a scenario readiness change"
    );

    let in_rehearsal = status == READY_STATUS;
    Ok(Json(ReadyChangeDto {
        message: readiness_message(&code, in_rehearsal),
        status,
        in_rehearsal,
    }))
}

/// The plain confirmation for a readiness change.
///
/// Pure, and separate from the handler, so the sentences a human reads before
/// deciding to demote a scenario are unit-testable without a database. §10's
/// language law puts every user-visible string on this side of the wire; this is
/// where these two are composed.
fn readiness_message(code: &str, in_rehearsal: bool) -> String {
    if in_rehearsal {
        format!("{code} is ready — Marie can now rehearse this scenario.")
    } else {
        // Says what did NOT change on purpose — see the handler's domain note.
        format!("{code} removed from rehearsal — nothing else changed.")
    }
}

/// `GET /cases/:slug/rehearsal` — every READY scenario, plus the standing card.
///
/// Open read (`AuthUser` establishes who is asking; the mode is not edit-gated —
/// rehearsing is reading). The gate is applied in the service, not here and
/// certainly not in the browser: a client cannot ask for a drafted scenario
/// because no parameter exists to ask with.
#[tracing::instrument(skip(state, _user), fields(slug = %slug))]
pub async fn get_rehearsal(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<RehearsalPayload>, AppError> {
    // One snapshot for the whole payload — the cap that trims each scenario's
    // points comes from the store (v2 §2b).
    let settings = state.settings.current();
    let payload = rehearsal_payload(&state.pipeline_pool, &slug, &settings)
        .await
        .map_err(readiness_error_to_app_error)?;

    tracing::info!(
        slug = %slug,
        ready_scenarios = payload.scenarios.len(),
        "served rehearsal mode"
    );

    Ok(Json(payload))
}

/// Map a [`ReadinessError`] onto its HTTP status.
///
/// The two refusals a HUMAN caused reach them verbatim; the two failures the
/// SERVER caused stay opaque and are logged with their cause. Same split as
/// `augmentation_error_to_app_error`.
fn readiness_error_to_app_error(error: ReadinessError) -> AppError {
    match error {
        ReadinessError::AlreadyInState { .. } => AppError::BadRequest {
            message: error.to_string(),
            details: json!({ "reason": "readiness_unchanged" }),
        },
        // A 404, not a 500: the scenario was deleted while the human had the page
        // open. Nothing is broken, and the message says exactly that.
        ReadinessError::Vanished => AppError::NotFound {
            message: error.to_string(),
        },
        ReadinessError::Read { .. } => {
            tracing::error!(error = %error, "failed to read for rehearsal");
            AppError::Internal {
                message: "failed to load".to_string(),
            }
        }
        ReadinessError::Write { .. } => {
            tracing::error!(error = %error, "failed to record a readiness change");
            AppError::Internal {
                message: "failed to save".to_string(),
            }
        }
    }
}

#[cfg(test)]
#[path = "rehearsal_tests.rs"]
mod tests;
