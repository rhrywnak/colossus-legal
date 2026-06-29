//! `GET /api/cases/:slug/trial-prep/dashboard` — the War Room dashboard payload.
//!
//! Thin vertical slice (see CC_WARROOM_WIRING_STEP2_BUILD): the response is a
//! full, valid `TrialPrepDashboard` in which exactly ONE number is graph-derived
//! — `marie-obstructive`'s `instance_count` (the ¶54 REBUTS count). Everything
//! else is the slice baseline produced by [`ScenarioDashboardAssembler`].
//!
//! Struct-repo handler pattern (precedent: `claims.rs`): construct a
//! `ScenarioRepository` from `state.graph.clone()`, hand it to the assembler,
//! return JSON. Observability + the bland 500 body mirror `proof_matrix.rs`.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use tracing::{error, info, instrument};

use crate::auth::AuthUser;
use crate::dto::trial_prep::TrialPrepDashboard;
use crate::repositories::scenario_repository::ScenarioRepository;
use crate::services::scenario_dashboard::ScenarioDashboardAssembler;
use crate::state::AppState;

/// Error type for this endpoint.
///
/// ## Why a single `Internal` variant (no 404)
///
/// Unlike `proof_matrix` (which 404s when the canonical structure is unloaded),
/// the dashboard ALWAYS returns a full baseline payload: an unloaded graph simply
/// yields `instance_count: 0` for the live card, which is a valid, observable
/// state — not "not found". The only failure is a genuine graph read error,
/// collapsed to an opaque 500 whose detail is logged for operators and never
/// returned to the client (Standing Rule 1).
pub enum TrialPrepEndpointError {
    /// Graph error or row-decode failure → HTTP 500 (logged, not returned).
    Internal,
}

/// 500 body: exactly `{"error":"internal server error"}` — nothing else.
#[derive(Serialize)]
struct InternalErrorBody {
    error: &'static str,
}

impl IntoResponse for TrialPrepEndpointError {
    fn into_response(self) -> Response {
        match self {
            TrialPrepEndpointError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(InternalErrorBody {
                    error: "internal server error",
                }),
            )
                .into_response(),
        }
    }
}

/// `GET /api/cases/:slug/trial-prep/dashboard`. No auth required, matching the
/// other case-scoped reads; the user is logged when present.
///
/// ## Why `slug` is accepted but not used to resolve the anchor
///
/// The `:slug` is bound for URL consistency with its sibling case routes
/// (`/cases/:slug/...`); dropping it would make this route an inconsistent
/// outlier. The slice does not need it to resolve the anchor — the anchor is the
/// hardcoded ¶54 id inside the assembler's scaffolding block. It is not dead,
/// though: it is recorded on the span (`fields(slug = …)`) and the access log
/// line, so the requested case is visible in traces until real slug→scenario
/// resolution lands.
#[instrument(skip(state, user), fields(slug = %slug))]
pub async fn get_trial_prep_dashboard(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<TrialPrepDashboard>, TrialPrepEndpointError> {
    if let Some(u) = &user {
        info!(username = %u.username, "GET /api/cases/{slug}/trial-prep/dashboard");
    }

    // Struct-repo pattern: Graph is Arc-backed, so the clone is cheap.
    let assembler = ScenarioDashboardAssembler::new(ScenarioRepository::new(state.graph.clone()));

    let dashboard = assembler
        .assemble()
        .await
        .map_err(internal("assemble trial-prep dashboard"))?;

    Ok(Json(dashboard))
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
        error!(error = %e, operation = op, "trial-prep dashboard request failed");
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
        let body = InternalErrorBody {
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
}
