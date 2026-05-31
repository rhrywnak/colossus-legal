//! `GET /api/cases/:slug/proof-matrix/rollup` — per-`LegalCount` deduped
//! allegation totals, the first piece of the Proof Matrix compute layer and the
//! single source of truth for Count-level deduped allegation counts. Neo4j only
//! (no Postgres).
//!
//! Thin handler: read the rollup rows via
//! [`crate::repositories::proof_matrix_repository`], map them straight into the
//! response DTO, return JSON. There is no builder/shaping step — the query
//! returns a flat, ordered list.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use tracing::{error, info, instrument};

use crate::auth::AuthUser;
use crate::dto::proof_matrix::{CountRollup, ProofMatrixRollupResponse};
use crate::repositories::proof_matrix_repository as repo;
use crate::state::AppState;

/// Error type for this endpoint.
///
/// ## Why a dedicated error instead of the shared `AppError`
///
/// Matching the causes-of-action endpoint (its direct neighbor), this read uses
/// its own error so the 500 body stays deliberately bland — graph/decode detail
/// is logged for operators, never returned to the client (Standing Rule 1). The
/// shared `AppError` body (`{"error","message","details"}`) would echo a
/// `message`, so it is intentionally not used here.
pub enum ProofMatrixEndpointError {
    /// No rollup rows exist → HTTP 404. The Neo4j graph is single-case and not
    /// slug-namespaced (and this endpoint touches no Postgres), so the slug is
    /// not validated against a case; an empty result means the canonical case
    /// structure (Counts / Elements / bearing Allegations) has not been loaded.
    NotFound { slug: String },
    /// Graph error or row-decode failure → HTTP 500 (logged, not returned).
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

impl IntoResponse for ProofMatrixEndpointError {
    fn into_response(self) -> Response {
        match self {
            ProofMatrixEndpointError::NotFound { slug } => (
                StatusCode::NOT_FOUND,
                Json(NotFoundBody {
                    error: "case structure not loaded",
                    slug,
                }),
            )
                .into_response(),
            ProofMatrixEndpointError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(InternalErrorBody {
                    error: "internal server error",
                }),
            )
                .into_response(),
        }
    }
}

/// `GET /api/cases/:slug/proof-matrix/rollup`. No auth required, matching the
/// other case-scoped reads; the user is logged when present.
///
/// ## Rust Learning: `Option<AuthUser>` as an extractor
///
/// `AuthUser` is an axum extractor that fails the request if the identity
/// headers are absent. Wrapping it in `Option` makes extraction infallible:
/// `Some(user)` when authenticated, `None` otherwise. This matches the
/// causes-of-action read, where auth is observed-and-logged but not enforced at
/// the handler (enforcement is upstream at Traefik/Authentik for UI routes).
#[instrument(skip(state, user), fields(slug = %slug))]
pub async fn get_proof_matrix_rollup(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<ProofMatrixRollupResponse>, ProofMatrixEndpointError> {
    if let Some(u) = &user {
        info!(username = %u.username, "GET /api/cases/{slug}/proof-matrix/rollup");
    }

    // Read the per-Count rollup. Zero rows ⇒ structure not loaded ⇒ 404.
    let rows = repo::fetch_rollup(&state.graph)
        .await
        .map_err(internal("fetch rollup"))?;
    if rows.is_empty() {
        return Err(ProofMatrixEndpointError::NotFound { slug });
    }

    // Map rows straight into the DTO (no builder — the list is already flat and
    // ordered by the query's ORDER BY).
    let counts = rows
        .into_iter()
        .map(|r| CountRollup {
            count_number: r.count_number,
            count_id: r.count_id,
            deduped_allegations: r.deduped_allegations,
        })
        .collect();

    Ok(Json(ProofMatrixRollupResponse {
        case_slug: slug,
        counts,
    }))
}

/// Map any displayable error to a logged `Internal` (500). `op` names the failed
/// step for the operator log; the client only ever sees the bland 500 body.
///
/// ## Rust Learning: returning `impl Fn(E) -> _` for `.map_err`
///
/// This returns a closure capturing `op`, so each call site reads
/// `.map_err(internal("fetch rollup"))?`. The closure logs with full context
/// (the underlying error and the operation) and collapses every failure into
/// the single opaque `Internal` variant — the place where the `?` chain
/// terminates at a handler that logs (Standing Rule 1).
fn internal<E: std::fmt::Display>(op: &'static str) -> impl Fn(E) -> ProofMatrixEndpointError {
    move |e| {
        error!(error = %e, operation = op, "proof-matrix rollup request failed");
        ProofMatrixEndpointError::Internal
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
    fn internal_server_error_body_does_not_contain_cypher_details() {
        let body = InternalErrorBody {
            error: "internal server error",
        };
        let value = serde_json::to_value(&body).expect("body serializes");
        assert_eq!(value, json!({"error": "internal server error"}));
        assert_eq!(value.as_object().expect("object body").len(), 1);
    }

    /// The 404 body echoes the requested slug so the caller can correlate which
    /// case was missing, with a stable `error` discriminant.
    #[test]
    fn not_found_body_includes_the_slug() {
        let body = NotFoundBody {
            error: "case structure not loaded",
            slug: "awad_v_catholic_family_service".to_string(),
        };
        assert_eq!(
            serde_json::to_value(&body).expect("body serializes"),
            json!({"error": "case structure not loaded", "slug": "awad_v_catholic_family_service"})
        );
    }

    /// `Internal` must map to HTTP 500. The body-shape tests above cannot catch
    /// a regression that swapped the status code in the `IntoResponse` match
    /// arm, so assert the status the variant attaches.
    #[test]
    fn internal_variant_maps_to_500() {
        let response = ProofMatrixEndpointError::Internal.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// `NotFound` must map to HTTP 404. This also exercises the handler's
    /// empty-rows → `NotFound` branch end state (the handler itself needs a live
    /// `AppState`, so the request path is covered by DEV verification; the
    /// status mapping is covered here).
    #[test]
    fn not_found_variant_maps_to_404() {
        let response = ProofMatrixEndpointError::NotFound {
            slug: "awad_v_catholic_family_service".to_string(),
        }
        .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
