//! `GET /api/cases/:slug/proof-review` — the read-only payload behind the
//! Proof-Review page's four sub-views (Summary, Proof edges, Excluded,
//! Borderline) over the `Evidence -[:CORROBORATES]-> Allegation` proof edges.
//! Neo4j only (no Postgres, no outbound HTTP).
//!
//! Thin handler: run the two reads via
//! [`crate::repositories::proof_review_repository`], hand the rows to
//! [`crate::repositories::proof_review_builder`] for shaping, return JSON. All
//! grouping/filtering lives in the builder; all schema knowledge lives in the
//! repository — the handler only wires extract → read → build → `Json`.
//!
//! ## No outbound-HTTP timeout here
//!
//! Standing Rule 13 ("every HTTP call has a timeout") does not apply: this
//! endpoint makes no outbound HTTP calls, only Neo4j reads over the shared
//! `state.graph` driver. Adding an unused `reqwest` timeout would be dead code,
//! so there is intentionally none.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument};

use crate::auth::AuthUser;
use crate::dto::proof_review::ProofReviewResponse;
use crate::repositories::proof_review_builder::build_proof_review;
use crate::repositories::proof_review_repository as repo;
use crate::state::AppState;

/// Optional query parameters. `document_id`, when present, scopes every
/// sub-view to that one source document; when absent, the payload spans all
/// documents.
///
/// ## Rust Learning: a typed `Query<T>` with an `Option` field
///
/// Deriving `Deserialize` on a struct of `Option` fields makes the whole query
/// string optional: a request with no `?document_id=` deserializes to
/// `document_id: None`. This mirrors `api::graph::GraphQuery` — the established
/// optional-query pattern in this codebase — rather than an untyped
/// `Query<HashMap<…>>`.
///
/// `deny_unknown_fields` makes a misspelled or stray query key (`?documentid=…`)
/// a loud 400 rather than a silently-ignored no-op — an unknown parameter and a
/// known one must not be indistinguishable (Standing Rule 1).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProofReviewParams {
    pub document_id: Option<String>,
}

/// Error type for this endpoint. Mirrors `proof_matrix::ProofMatrixEndpointError`.
///
/// ## Why a dedicated error instead of the shared `AppError`
///
/// As with the proof-matrix and causes-of-action reads, this endpoint uses its
/// own error so the 500 body stays deliberately bland — graph/decode detail is
/// logged for operators, never returned to the client (Standing Rule 1).
pub enum ProofReviewEndpointError {
    /// Both reads returned nothing → HTTP 404. The Neo4j graph is single-case
    /// and not slug-namespaced, so the slug is not validated against a case; an
    /// all-empty result means no discovery proof data is loaded for the
    /// requested scope (no corroborating edges and no preserved non-answers).
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

impl IntoResponse for ProofReviewEndpointError {
    fn into_response(self) -> Response {
        match self {
            ProofReviewEndpointError::NotFound { slug } => (
                StatusCode::NOT_FOUND,
                Json(NotFoundBody {
                    error: "no proof-review data",
                    slug,
                }),
            )
                .into_response(),
            ProofReviewEndpointError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(InternalErrorBody {
                    error: "internal server error",
                }),
            )
                .into_response(),
        }
    }
}

/// `GET /api/cases/:slug/proof-review`. No auth required, matching the other
/// case-scoped reads; the user is logged when present. The graph read is NOT
/// filtered by `slug` — it is used only for the log line, span, and 404, exactly
/// like `get_proof_matrix_rollup` (single-case-implicit graph).
#[instrument(skip(state, user), fields(slug = %slug, document_id = tracing::field::Empty))]
pub async fn get_proof_review(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<ProofReviewParams>,
) -> Result<Json<ProofReviewResponse>, ProofReviewEndpointError> {
    if let Some(doc) = &params.document_id {
        tracing::Span::current().record("document_id", tracing::field::display(doc));
    }
    if let Some(u) = &user {
        info!(
            username = %u.username,
            document_id = ?params.document_id,
            "GET /api/cases/{slug}/proof-review"
        );
    }

    // Two read-only graph queries, optionally scoped to one document.
    let document_id = params.document_id.as_deref();
    let edge_rows = repo::fetch_proof_edges(&state.graph, document_id)
        .await
        .map_err(internal("fetch proof edges"))?;
    let excluded_rows = repo::fetch_excluded(&state.graph, document_id)
        .await
        .map_err(internal("fetch excluded"))?;

    // No corroborating edges AND no preserved non-answers ⇒ nothing loaded for
    // this scope ⇒ 404 (distinct from a query error, which is a logged 500).
    if edge_rows.is_empty() && excluded_rows.is_empty() {
        return Err(ProofReviewEndpointError::NotFound { slug });
    }

    let response = build_proof_review(slug, params.document_id, edge_rows, excluded_rows);
    Ok(Json(response))
}

/// Map any displayable error to a logged `Internal` (500). `op` names the failed
/// step for the operator log; the client only ever sees the bland 500 body.
///
/// ## Rust Learning: returning `impl Fn(E) -> _` for `.map_err`
///
/// This returns a closure capturing `op`, so each call site reads
/// `.map_err(internal("fetch proof edges"))?`. The closure logs with full
/// context (the underlying error and the operation) and collapses every failure
/// into the single opaque `Internal` variant — the place where the `?` chain
/// terminates at a handler that logs (Standing Rule 1).
fn internal<E: std::fmt::Display>(op: &'static str) -> impl Fn(E) -> ProofReviewEndpointError {
    move |e| {
        error!(error = %e, operation = op, "proof-review request failed");
        ProofReviewEndpointError::Internal
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
    fn internal_server_error_body_does_not_contain_details() {
        let body = InternalErrorBody {
            error: "internal server error",
        };
        let value = serde_json::to_value(&body).expect("body serializes");
        assert_eq!(value, json!({"error": "internal server error"}));
        assert_eq!(value.as_object().expect("object body").len(), 1);
    }

    /// The 404 body echoes the requested slug so the caller can correlate which
    /// case was empty, with a stable `error` discriminant.
    #[test]
    fn not_found_body_includes_the_slug() {
        let body = NotFoundBody {
            error: "no proof-review data",
            slug: "awad_v_catholic_family_service".to_string(),
        };
        assert_eq!(
            serde_json::to_value(&body).expect("body serializes"),
            json!({"error": "no proof-review data", "slug": "awad_v_catholic_family_service"})
        );
    }

    /// `Internal` must map to HTTP 500 — a regression that swapped the status in
    /// the `IntoResponse` match arm would not be caught by the body-shape tests.
    #[test]
    fn internal_variant_maps_to_500() {
        let response = ProofReviewEndpointError::Internal.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// `NotFound` must map to HTTP 404. This also exercises the handler's
    /// all-empty → `NotFound` branch end state (the handler itself needs a live
    /// `AppState`, so the request path is covered by DEV verification; the
    /// status mapping is covered here).
    #[test]
    fn not_found_variant_maps_to_404() {
        let response = ProofReviewEndpointError::NotFound {
            slug: "awad_v_catholic_family_service".to_string(),
        }
        .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    /// `document_id` is an optional field: an absent param deserializes to
    /// `None`, a present one to `Some(value)`. Guards the optional-filter
    /// contract — a regression making the field required would reject every
    /// unfiltered request. (Serialized here via serde_json, a direct dependency;
    /// the urlencoded `Query` path is the same serde derive, exercised live.)
    #[test]
    fn params_field_is_optional() {
        let none: ProofReviewParams =
            serde_json::from_value(json!({})).expect("absent document_id deserializes");
        assert_eq!(none.document_id, None);
        let some: ProofReviewParams = serde_json::from_value(json!({"document_id": "doc-george"}))
            .expect("present document_id deserializes");
        assert_eq!(some.document_id.as_deref(), Some("doc-george"));
    }
}
