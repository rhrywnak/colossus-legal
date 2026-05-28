//! Element detail panel handlers.
//!
//! Two endpoints:
//!
//! - `GET  /api/cases/:slug/elements/:element_id/detail` — composite read of
//!   the Element node (Neo4j), its parent LegalCount (Neo4j), every mapped
//!   Allegation (Neo4j), and the human-authored `review_notes` (Postgres).
//! - `PATCH /api/cases/:slug/elements/:element_id/notes` — persist or clear
//!   `review_notes` for the Element. Body is `{"review_notes": "…" | null}`.
//!
//! Both endpoints are unauthenticated, matching the surrounding read/write
//! endpoints in [`crate::api`].
//!
//! ## Why the `:slug` path segment is unused
//!
//! The Neo4j graph is single-case and the `entity_id` is globally unique, so
//! we don't filter Postgres or Neo4j by slug. The segment is URL-shape
//! convention only — keeps the public URL aligned with
//! `/api/cases/:slug/causes-of-action` and leaves room for a per-case scoping
//! filter if we ever onboard a second case.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info, instrument};

use crate::auth::AuthUser;
use crate::repositories::element_detail_repository::{
    fetch_element_with_allegations, ElementDetailRepoError, ElementDetailResponse,
};
use crate::repositories::pipeline_repository::{authored_entities, PipelineRepoError};
use crate::state::AppState;

// ── Error mapping ─────────────────────────────────────────────────

/// Endpoint-local error type. The detail panel design needs an explicit 404
/// for "Element not found" without leaking Cypher / SQL into the body, so we
/// don't reuse the shared [`crate::error::AppError`] here — same rationale as
/// `causes_of_action::CausesEndpointError`.
pub enum ElementDetailEndpointError {
    /// Element id missing from Neo4j (or, on PATCH, from `authored_entities`).
    /// → HTTP 404.
    NotFound { element_id: String },
    /// Anything else — graph error, decode failure, Postgres error.
    /// → HTTP 500. Detail is logged for the operator, not returned.
    Internal,
}

#[derive(Serialize)]
struct NotFoundBody {
    error: &'static str,
    element_id: String,
}

#[derive(Serialize)]
struct InternalErrorBody {
    error: &'static str,
}

impl IntoResponse for ElementDetailEndpointError {
    fn into_response(self) -> Response {
        match self {
            ElementDetailEndpointError::NotFound { element_id } => (
                StatusCode::NOT_FOUND,
                Json(NotFoundBody {
                    error: "element not found",
                    element_id,
                }),
            )
                .into_response(),
            ElementDetailEndpointError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(InternalErrorBody {
                    error: "internal server error",
                }),
            )
                .into_response(),
        }
    }
}

// ── GET handler ───────────────────────────────────────────────────

/// `GET /api/cases/:slug/elements/:element_id/detail`.
///
/// Returns the [`ElementDetailResponse`] payload on 200, a 404 with the
/// element_id on miss, or a bland 500 on any backend error (logged with
/// operation + source).
#[instrument(skip(state, user), fields(slug = %slug, element_id = %element_id))]
pub async fn get_element_detail(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path((slug, element_id)): Path<(String, String)>,
) -> Result<Json<ElementDetailResponse>, ElementDetailEndpointError> {
    // Log the request unconditionally — an unauthenticated caller still
    // needs to leave an operator-visible trail. The anonymous fallback
    // keeps the `username` field present in every span so log queries can
    // group on it without special-casing missing values.
    let username = user
        .as_ref()
        .map(|u| u.username.as_str())
        .unwrap_or("<anonymous>");
    info!(
        username = username,
        "GET /api/cases/{slug}/elements/{element_id}/detail"
    );

    match fetch_element_with_allegations(&state.graph, &state.pipeline_pool, &element_id).await {
        Ok(detail) => Ok(Json(detail)),
        Err(ElementDetailRepoError::NotFound { element_id }) => {
            // Distinct observable: tell the operator log this was a real miss
            // (no Element with that id), not a backend error.
            info!(
                element_id = %element_id,
                "Element detail miss — no node with that id"
            );
            Err(ElementDetailEndpointError::NotFound { element_id })
        }
        Err(e) => {
            // Log the full error chain (`{e:#}` walks `#[source]`) so the
            // operator can see Cypher / SQL detail without it leaking to the
            // client.
            error!(
                error = ?e,
                error_display = %e,
                "element-detail GET failed"
            );
            Err(ElementDetailEndpointError::Internal)
        }
    }
}

// ── PATCH handler ─────────────────────────────────────────────────

/// Request body for the notes PATCH. `review_notes = None` (JSON `null`) and
/// `review_notes = Some("")` are intentionally distinguishable: the former
/// clears the column, the latter writes an empty string. Tests pin the
/// behavior.
///
/// `#[serde(default)]` means a missing `review_notes` key decodes to `None`,
/// matching JSON `null` — convenient for clients that want to clear notes
/// with an empty body `{}`.
///
/// `#[serde(deny_unknown_fields)]` rejects extra keys at the body's top
/// level. A client that accidentally sends e.g. `{"reviewNotes": "..."}`
/// (camelCase typo) gets a 4xx parse error rather than a silently-ignored
/// field and the wrong column written. Same posture as
/// `repositories::pipeline_repository::config::PipelineConfigInput`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateNotesRequest {
    #[serde(default)]
    pub review_notes: Option<String>,
}

/// `PATCH /api/cases/:slug/elements/:element_id/notes`.
///
/// Returns `200 { "status": "saved" }` on success, `404` if the Element row
/// doesn't exist in `authored_entities`, `500` on any other failure.
#[instrument(skip(state, user, payload), fields(slug = %slug, element_id = %element_id))]
pub async fn patch_element_notes(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path((slug, element_id)): Path<(String, String)>,
    Json(payload): Json<UpdateNotesRequest>,
) -> Result<Json<serde_json::Value>, ElementDetailEndpointError> {
    // Log unconditionally so anonymous PATCHes still produce an operator-
    // visible trail. `action` distinguishes set-vs-clear (operationally
    // distinct states, Rule 1); the notes contents are deliberately NOT
    // logged because they could include attorney-client privileged matter.
    let username = user
        .as_ref()
        .map(|u| u.username.as_str())
        .unwrap_or("<anonymous>");
    let action = if payload.review_notes.is_some() {
        "set"
    } else {
        "clear"
    };
    info!(
        username = username,
        action = action,
        "PATCH /api/cases/{slug}/elements/{element_id}/notes"
    );

    let notes_borrow: Option<&str> = payload.review_notes.as_deref();

    match authored_entities::update_element_review_notes(
        &state.pipeline_pool,
        &element_id,
        notes_borrow,
    )
    .await
    {
        Ok(()) => Ok(Json(json!({ "status": "saved" }))),
        Err(PipelineRepoError::NotFound(missing)) => {
            info!(
                element_id = %element_id,
                missing = %missing,
                "PATCH notes miss — no authored_entities row"
            );
            Err(ElementDetailEndpointError::NotFound { element_id })
        }
        Err(e) => {
            error!(
                error = ?e,
                error_display = %e,
                "element-detail PATCH failed"
            );
            Err(ElementDetailEndpointError::Internal)
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json as j;

    /// Pin the 404 body shape. The frontend keys on this exact JSON.
    #[test]
    fn not_found_body_shape() {
        let body = NotFoundBody {
            error: "element not found",
            element_id: "element-1-1".to_string(),
        };
        assert_eq!(
            serde_json::to_value(&body).unwrap(),
            j!({"error": "element not found", "element_id": "element-1-1"})
        );
    }

    /// Pin the bland 500 body — no Cypher / SQL leakage.
    #[test]
    fn internal_body_shape_is_bland() {
        let body = InternalErrorBody {
            error: "internal server error",
        };
        let value = serde_json::to_value(&body).unwrap();
        assert_eq!(value, j!({"error": "internal server error"}));
        assert_eq!(value.as_object().unwrap().len(), 1);
    }

    /// JSON `null` decodes to `None` (clear notes).
    #[test]
    fn update_notes_request_decodes_null_as_none() {
        let req: UpdateNotesRequest =
            serde_json::from_value(j!({"review_notes": null})).expect("decodes");
        assert_eq!(req.review_notes, None);
    }

    /// Empty body decodes to `None` via `#[serde(default)]` — same effect as
    /// explicit null, by design.
    #[test]
    fn update_notes_request_decodes_missing_key_as_none() {
        let req: UpdateNotesRequest = serde_json::from_value(j!({})).expect("decodes");
        assert_eq!(req.review_notes, None);
    }

    /// Empty string is NOT the same as null — it persists as empty notes
    /// (Rule 1: distinct observable states).
    #[test]
    fn update_notes_request_distinguishes_empty_string_from_null() {
        let req: UpdateNotesRequest =
            serde_json::from_value(j!({"review_notes": ""})).expect("decodes");
        assert_eq!(req.review_notes, Some(String::new()));
    }
}
