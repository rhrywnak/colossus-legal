//! `GET /api/cases/:slug/trial-prep/dashboard` — the War Room dashboard payload.
//!
//! The response is a full `TrialPrepDashboard` assembled by
//! [`ScenarioDashboardAssembler`] from the case's real `scenarios` rows
//! (Postgres pipeline DB) plus each card's live REBUTS count from the graph.
//! With no scenarios authored yet the dashboard is honestly empty (no cards,
//! zeroed metrics, no alerts).
//!
//! Handler pattern (precedent: `claims.rs`): build the assembler from
//! `state.graph.clone()` + `state.pipeline_pool.clone()`, call it with the case
//! slug, return JSON. Observability + the bland 500 body mirror `proof_matrix.rs`.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::dto::trial_prep::{ScenarioDetail, TrialPrepDashboard};
use crate::repositories::scenario_repository::ScenarioRepository;
use crate::services::scenario_dashboard::ScenarioDashboardAssembler;
use crate::state::AppState;

/// Error type for the trial-prep endpoints.
///
/// The dashboard read only ever fails internally (it always has a valid payload
/// otherwise), so it uses `Internal`. The scenario-detail read adds two client
/// errors: a malformed `scenario_id` (`BadRequest`) and an absent scenario
/// (`NotFound` — the legitimate deleted/unknown id). Every 5xx detail is logged
/// for operators and never returned to the client (Standing Rule 1); the 4xx
/// bodies carry only a short, non-sensitive reason.
pub enum TrialPrepEndpointError {
    /// Graph/store error or row-decode failure → HTTP 500 (logged, not returned).
    Internal,
    /// No scenario with the requested id → HTTP 404.
    NotFound,
    /// A malformed path value (e.g. a non-UUID scenario id) → HTTP 400.
    BadRequest { reason: &'static str },
}

/// Error body: a single `error` string. Reused for the 400/404/500 responses; it
/// never carries Cypher, a store cause, or a `details` object (Standing Rule 1 —
/// no leak to the client).
#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
}

impl IntoResponse for TrialPrepEndpointError {
    fn into_response(self) -> Response {
        match self {
            TrialPrepEndpointError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "internal server error",
                }),
            )
                .into_response(),
            TrialPrepEndpointError::NotFound => (
                StatusCode::NOT_FOUND,
                Json(ErrorBody {
                    error: "scenario not found",
                }),
            )
                .into_response(),
            TrialPrepEndpointError::BadRequest { reason } => {
                (StatusCode::BAD_REQUEST, Json(ErrorBody { error: reason })).into_response()
            }
        }
    }
}

/// `GET /api/cases/:slug/trial-prep/dashboard`. No auth required, matching the
/// other case-scoped reads; the user is logged when present.
///
/// `slug` selects which case's scenarios to assemble — it is passed to the
/// assembler, which lists that case's rows from Postgres and computes each card's
/// live count from the graph.
#[instrument(skip(state, user), fields(slug = %slug))]
pub async fn get_trial_prep_dashboard(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<TrialPrepDashboard>, TrialPrepEndpointError> {
    if let Some(u) = &user {
        info!(username = %u.username, "GET /api/cases/{slug}/trial-prep/dashboard");
    }

    // Both handles are cheap clones (Graph and PgPool are each Arc-backed).
    let assembler = ScenarioDashboardAssembler::new(
        ScenarioRepository::new(state.graph.clone()),
        state.pipeline_pool.clone(),
    );

    let dashboard = assembler
        .assemble(&slug)
        .await
        .map_err(internal("assemble trial-prep dashboard"))?;

    Ok(Json(dashboard))
}

/// `GET /api/cases/:slug/trial-prep/scenarios/:scenario_id` — one scenario's
/// detail (its record + its anchor allegations' graph evidence as a timeline).
///
/// `slug` selects the case (logged); the scenario is identified by the
/// globally-unique `scenario_id`. A malformed id → 400; an absent scenario → 404
/// (the legitimate deleted/unknown id); a store/graph failure → a logged 500.
#[instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn get_trial_prep_scenario_detail(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<ScenarioDetail>, TrialPrepEndpointError> {
    if let Some(u) = &user {
        info!(username = %u.username, "GET /api/cases/{slug}/trial-prep/scenarios/{scenario_id}");
    }

    // A malformed uuid is a client error (400), not a server fault.
    let id = Uuid::parse_str(&scenario_id).map_err(|_| TrialPrepEndpointError::BadRequest {
        reason: "scenario_id must be a valid UUID",
    })?;

    let assembler = ScenarioDashboardAssembler::new(
        ScenarioRepository::new(state.graph.clone()),
        state.pipeline_pool.clone(),
    );

    // `None` from the assembler ⇒ no such scenario row ⇒ 404 (distinct from a
    // store/graph error, which the `internal` closure collapses to a logged 500).
    assembler
        .assemble_detail(id)
        .await
        .map_err(internal("assemble scenario detail"))?
        .map(Json)
        .ok_or(TrialPrepEndpointError::NotFound)
}

/// Map any displayable error to a logged `Internal` (500). `op` names the failed
/// step for the operator log; the client only ever sees the bland 500 body.
///
/// ## Rust Learning: returning `impl Fn(E) -> _` for `.map_err`
///
/// Mirrors `proof_matrix::internal`: the returned closure captures `op`, logs
/// the underlying error with full context, and collapses every failure into the
/// single opaque `Internal` variant — the place where the `?` chain terminates
/// at a handler that logs (Standing Rule 1).
fn internal<E: std::fmt::Display>(op: &'static str) -> impl Fn(E) -> TrialPrepEndpointError {
    move |e| {
        // `op` (the `operation` field) carries the specific step; the message
        // stays generic so it never contradicts the operation (this closure is
        // shared by the dashboard and scenario-detail handlers).
        error!(error = %e, operation = op, "trial-prep request failed");
        TrialPrepEndpointError::Internal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The 500 body must never carry Cypher, an error message, or a `details`
    /// object — exactly one key, `error`. Guards Standing Rule 1's "no leak to
    /// the client" requirement at the serialization boundary.
    #[test]
    fn internal_server_error_body_does_not_leak_detail() {
        let body = ErrorBody {
            error: "internal server error",
        };
        let value = serde_json::to_value(&body).expect("body serializes");
        assert_eq!(value, json!({"error": "internal server error"}));
        assert_eq!(value.as_object().expect("object body").len(), 1);
    }

    /// `Internal` must map to HTTP 500. The body-shape test above cannot catch a
    /// regression that swapped the status code in the `IntoResponse` match arm.
    #[test]
    fn internal_variant_maps_to_500() {
        let response = TrialPrepEndpointError::Internal.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// `NotFound` (absent scenario) must map to HTTP 404.
    #[test]
    fn not_found_variant_maps_to_404() {
        let response = TrialPrepEndpointError::NotFound.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    /// `BadRequest` (malformed uuid) must map to HTTP 400 and echo only the bland
    /// reason — no internal detail.
    #[test]
    fn bad_request_variant_maps_to_400() {
        let response = TrialPrepEndpointError::BadRequest {
            reason: "scenario_id must be a valid UUID",
        }
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
