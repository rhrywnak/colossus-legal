//! `GET /api/cases/:slug` — read-only case header for the redesigned Home page.
//!
//! Thin handler: resolve the slug, read three tables via
//! [`crate::repositories::case_header_repository`], shape them with
//! [`crate::repositories::case_header_builder::build_case_header`], return JSON.
//! No writes, no Neo4j.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use tracing::{error, info, instrument};

use crate::auth::AuthUser;
use crate::dto::case_header::CaseHeaderResponse;
use crate::repositories::case_header_builder::build_case_header;
use crate::repositories::case_header_repository as repo;
use crate::state::AppState;

/// Error type for this endpoint.
///
/// ## Why a dedicated error instead of the shared `AppError`
///
/// The Home page design spec prescribes exact response bodies here —
/// `{"error":"case not found","slug":"<slug>"}` (404) and
/// `{"error":"internal server error"}` (500). The shared `AppError`'s
/// `IntoResponse` emits a different shape (`{"error","message","details"}`)
/// and cannot produce these. The frontend (instruction 4) is built against the
/// exact shapes, so we keep a small local error here rather than widen the
/// shared type. The 500 body is deliberately bland: database/SQL detail is
/// logged for operators, never returned to the client.
pub enum CaseEndpointError {
    /// No case matches the slug → HTTP 404 (carries the slug for the body).
    NotFound { slug: String },
    /// Any database or shaping failure → HTTP 500 (details logged, not returned).
    Internal,
}

/// 404 body: a fixed message plus the slug that was looked up.
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

impl IntoResponse for CaseEndpointError {
    fn into_response(self) -> Response {
        match self {
            CaseEndpointError::NotFound { slug } => (
                StatusCode::NOT_FOUND,
                Json(NotFoundBody {
                    error: "case not found",
                    slug,
                }),
            )
                .into_response(),
            CaseEndpointError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(InternalErrorBody {
                    error: "internal server error",
                }),
            )
                .into_response(),
        }
    }
}

/// `GET /api/cases/:slug` — title, court, status, parties (plaintiffs /
/// active defendants / dropped defendants), and counsel. No auth, matching the
/// other read handlers.
#[instrument(skip(state, user), fields(slug = %slug))]
pub async fn get_case_by_slug(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<CaseHeaderResponse>, CaseEndpointError> {
    if let Some(u) = &user {
        info!(username = %u.username, "GET /api/cases/{slug}");
    }

    // 1. The case itself: absent → 404; DB error → 500 (logged with context).
    let case = repo::fetch_case_by_slug(&state.pg_pool, &slug)
        .await
        .map_err(internal("fetch case by slug"))?
        .ok_or(CaseEndpointError::NotFound { slug: slug.clone() })?;

    // 2 & 3. Parties and counsel for the resolved case_id (reads only).
    let parties = repo::fetch_parties(&state.pg_pool, &case.case_id)
        .await
        .map_err(internal("fetch parties"))?;
    let counsel = repo::fetch_counsel(&state.pg_pool, &case.case_id)
        .await
        .map_err(internal("fetch counsel"))?;

    // 4. Shape into the response (bucketing/sorting; rejects an invalid role).
    let response =
        build_case_header(case, parties, counsel).map_err(internal("shape case header"))?;

    Ok(Json(response))
}

/// Map any displayable error to a logged `Internal` (500). `op` names the
/// failed step for the operator-facing log; the client only ever sees the
/// bland 500 body (Standing Rule 1: context in logs, not on the wire).
fn internal<E: std::fmt::Display>(op: &'static str) -> impl Fn(E) -> CaseEndpointError {
    move |e| {
        error!(error = %e, operation = op, "case header request failed");
        CaseEndpointError::Internal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn error_response_for_unknown_slug_includes_the_slug() {
        let body = NotFoundBody {
            error: "case not found",
            slug: "no-such-case".to_string(),
        };
        assert_eq!(
            serde_json::to_value(&body).unwrap(),
            json!({"error": "case not found", "slug": "no-such-case"})
        );
    }

    #[test]
    fn internal_server_error_body_does_not_contain_sql_details() {
        let body = InternalErrorBody {
            error: "internal server error",
        };
        let value = serde_json::to_value(&body).unwrap();
        assert_eq!(value, json!({"error": "internal server error"}));
        // Exactly one key — no `message`, `details`, or SQL text can leak.
        assert_eq!(value.as_object().unwrap().len(), 1);
    }
}
