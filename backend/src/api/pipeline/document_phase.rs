//! Recording which phase of the case a document belongs to (task DOCUMENT_PHASE).
//!
//! ## One endpoint, two callers — the `document_date` shape
//!
//! Deliberately the same shape as its sibling `document_date`: one write, called
//! by the upload dialog after the file lands and again by the document page
//! whenever Roman corrects it. Two callers, one validation. A second write path
//! would eventually validate differently, and the difference would be invisible
//! until a document sat in the wrong phase.
//!
//! ## The slug goes out, the label never does
//!
//! Ruled 2026-08-17: display labels (PRE-PROBATE · PROBATE · COA · COMPLAINT)
//! live in `frontend/public/data/timeline.json` and are read from there by every
//! surface that renders one. This handler returns the slug and nothing else.
//!
//! That is why there is no `phase_label` field here, and no `GET /phases`
//! vocabulary endpoint of the kind `document_date` has for its precisions. The
//! date's precisions carry a backend rule — which of them require a date — so
//! the frontend must ask. A phase carries no rule at all: it is four names, and
//! the frontend already holds them, in the file it renders the timeline from.
//! Serving a second copy from here would create the drift the ruling exists to
//! prevent.
//!
//! ## Absence is an answer
//!
//! The phase is never required (chronology design R4). Clearing it is a normal
//! operation, not an error — which is the one place this differs from the date's
//! mandatory-with-override rule. See `domain::case_phase::validate`.

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::domain::case_phase::{validate, CasePhaseError};
use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository;
use crate::state::AppState;

/// What the intake dialog and the document page send.
///
/// `deny_unknown_fields` because this is a request body from a browser: a field
/// name the backend does not know is a client that has drifted, and reading it as
/// "no phase supplied" would silently clear a phase somebody had set.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetDocumentPhaseRequest {
    /// The slug — `estate` | `probate` | `appeals` | `civil_lawsuit`. Absent,
    /// null or empty clears the field.
    #[serde(default)]
    pub phase: Option<String>,
}

/// What either caller gets back — the stored state, read back rather than echoed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentPhaseResponse {
    pub document_id: String,
    /// The slug, or absent when the document has no phase recorded. No label:
    /// see the module header.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
}

/// `PUT /documents/:id/phase` — set or clear a document's phase.
///
/// ## Why PUT and not PATCH
///
/// The task said "PATCH on the existing document-update route (the same one
/// document_date uses)". There is no general document-update route: the date is
/// its own `PUT /documents/:id/date`. Matching the sibling exactly was the
/// instruction's evident intent, so this is `PUT /documents/:id/phase` — and PUT
/// is the honest verb regardless, because the body carries the field's whole
/// value and sending `{"phase": null}` is how you clear it.
pub async fn set_document_phase(
    user: AuthUser,
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    Json(body): Json<SetDocumentPhaseRequest>,
) -> Result<Json<DocumentPhaseResponse>, AppError> {
    require_admin(&user)?;

    let phase = validate(body.phase.as_deref()).map_err(|e| match e {
        CasePhaseError::Unknown { .. } => AppError::BadRequest {
            message: format!("Cannot set the phase for document '{document_id}': {e}"),
            details: serde_json::json!({ "field": "phase" }),
        },
    })?;
    let slug = phase.map(|p| p.slug());

    let rows = pipeline_repository::set_document_phase(&state.pipeline_pool, &document_id, slug)
        .await
        .map_err(|source| {
            tracing::error!(
                document_id = %document_id,
                phase = ?slug,
                error = %source,
                "could not store the document phase"
            );
            AppError::Internal {
                message: format!(
                    "Failed to store the phase for document '{document_id}': {source}"
                ),
            }
        })?;

    // An UPDATE that matched nothing is not an error to Postgres and would not be
    // one to `?` either — it simply changed zero rows. Discarding the count here
    // would mean a phase set against a document id that does not exist returned
    // 200 with the value echoed back and stored nothing. Standing Rule 1: "no
    // such document" and "stored" are two operationally distinct states, so they
    // get two observables. Same reasoning as `set_document_date`.
    if rows == 0 {
        tracing::warn!(
            document_id = %document_id,
            "refused a document phase: no such document. Nothing was stored"
        );
        return Err(AppError::NotFound {
            message: format!("Document '{document_id}' not found — the phase was not stored"),
        });
    }

    // A curatorial act on the case record, not a setting: which phase a document
    // belongs to governs where it sits in the timeline and which filtered views
    // it appears in. "Who assigned this and when?" has to be answerable from
    // `admin_audit_log` months later — a tracing line is transient, unqueryable
    // from any screen, and gone with the container.
    log_admin_action(
        &state.audit_repo,
        &user.username,
        "pipeline.document.set_phase",
        Some("document"),
        Some(&document_id),
        Some(serde_json::json!({ "phase": slug })),
    )
    .await;

    tracing::info!(
        document_id = %document_id,
        phase = ?slug,
        user = %user.username,
        "document phase recorded"
    );

    Ok(Json(DocumentPhaseResponse {
        document_id,
        phase: slug.map(str::to_string),
    }))
}

#[cfg(test)]
#[path = "document_phase_tests.rs"]
mod tests;
