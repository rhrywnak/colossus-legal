//! `GET /api/cases/:slug/causes-of-action` — Counts + canonical Elements for
//! the redesigned Home page Causes of Action tables (`HOME_PAGE_REDESIGN_v2.md`
//! §7). Neo4j only (no Postgres).
//!
//! Thin handler: read Counts and Elements via
//! [`crate::repositories::causes_of_action_repository`], shape them with
//! [`crate::repositories::causes_of_action_builder::build_causes_of_action`],
//! return JSON.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use tracing::{error, info, instrument};

use crate::auth::AuthUser;
use crate::dto::causes_of_action::CausesOfActionResponse;
use crate::repositories::causes_of_action_builder::build_causes_of_action;
use crate::repositories::causes_of_action_repository as repo;
use crate::state::AppState;

/// Error type for this endpoint.
///
/// ## Why a dedicated error instead of the shared `AppError`
///
/// Like the case-header endpoint, the design spec prescribes exact bodies that
/// the shared `AppError` (`{"error","message","details"}`) cannot produce. The
/// 500 body is deliberately bland — graph/decode detail is logged for
/// operators, never returned to the client.
pub enum CausesEndpointError {
    /// No `LegalCount` nodes exist → HTTP 404. The Neo4j graph is single-case
    /// and not slug-namespaced (and this endpoint touches no Postgres), so the
    /// slug isn't validated against a case; an empty graph means the canonical
    /// case structure hasn't been loaded yet.
    NotFound { slug: String },
    /// Graph error or malformed JSON property → HTTP 500 (logged, not returned).
    Internal,
}

/// 404 body.
#[derive(Serialize)]
struct NotFoundBody {
    error: &'static str,
    slug: String,
}

/// 500 body: exactly `{"error":"internal server error"}` — nothing else.
#[derive(Serialize)]
struct InternalErrorBody {
    error: &'static str,
}

impl IntoResponse for CausesEndpointError {
    fn into_response(self) -> Response {
        match self {
            CausesEndpointError::NotFound { slug } => (
                StatusCode::NOT_FOUND,
                Json(NotFoundBody {
                    error: "case structure not loaded",
                    slug,
                }),
            )
                .into_response(),
            CausesEndpointError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(InternalErrorBody {
                    error: "internal server error",
                }),
            )
                .into_response(),
        }
    }
}

/// `GET /api/cases/:slug/causes-of-action`. No auth, matching other reads.
#[instrument(skip(state, user), fields(slug = %slug))]
pub async fn get_causes_of_action(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<CausesOfActionResponse>, CausesEndpointError> {
    if let Some(u) = &user {
        info!(username = %u.username, "GET /api/cases/{slug}/causes-of-action");
    }

    // 1. Counts. Zero LegalCount nodes ⇒ structure not loaded ⇒ 404.
    let counts = repo::fetch_counts(&state.graph)
        .await
        .map_err(internal("fetch counts"))?;
    if counts.is_empty() {
        return Err(CausesEndpointError::NotFound { slug });
    }

    // 2. Elements (one query for all Counts; joined by count_number in step 3).
    let elements = repo::fetch_elements(&state.graph)
        .await
        .map_err(internal("fetch elements"))?;

    // 3. Shape: group/sort + decode JSON properties (malformed JSON → 500).
    let response = build_causes_of_action(&slug, counts, elements)
        .map_err(internal("shape causes of action"))?;

    Ok(Json(response))
}

/// Map any displayable error to a logged `Internal` (500). `op` names the
/// failed step for the operator log; the client only sees the bland 500 body.
fn internal<E: std::fmt::Display>(op: &'static str) -> impl Fn(E) -> CausesEndpointError {
    move |e| {
        error!(error = %e, operation = op, "causes-of-action request failed");
        CausesEndpointError::Internal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn internal_server_error_body_does_not_contain_cypher_details() {
        let body = InternalErrorBody {
            error: "internal server error",
        };
        let value = serde_json::to_value(&body).unwrap();
        assert_eq!(value, json!({"error": "internal server error"}));
        // Exactly one key — no Cypher text, no `message`/`details` can leak.
        assert_eq!(value.as_object().unwrap().len(), 1);
    }

    #[test]
    fn not_found_body_includes_the_slug() {
        let body = NotFoundBody {
            error: "case structure not loaded",
            slug: "awad_v_catholic_family_service".to_string(),
        };
        assert_eq!(
            serde_json::to_value(&body).unwrap(),
            json!({"error": "case structure not loaded", "slug": "awad_v_catholic_family_service"})
        );
    }
}
