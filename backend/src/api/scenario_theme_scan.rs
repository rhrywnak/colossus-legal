//! Theme Scan HTTP route (D2b).
//!
//! One `POST` route that runs the LLM judge over every candidate quote about a
//! scenario's subject and records every verdict to `scan_run_verdicts`, plus the
//! reads (poll, history) and the one write (delete) around a run.
//!
//! The judgment logic lives in `services::theme_scan`; this module is a thin
//! transport shell — extract, authorize, delegate, map the typed service error
//! onto an HTTP status.
//!
//! ## The scan does NOT write candidate facts
//!
//! An earlier version of this doc said the scan "persists the relevant verdicts as
//! `confirmed=false` suggestions". Both halves are now wrong: the `confirmed`
//! column was replaced by `status` in migration 20260706162558, and a scan writes
//! nothing to `scenario_fact_refs` at all. Scanning SCORES.
//!
//! What a completed run's verdicts DO is show up in the candidate queue as
//! proposals — a read-time projection composed by `api::scenario_cards`, which
//! writes nothing either. The human's ruling is the one write path into a
//! scenario's candidate facts (2026-08-08; the **Merge selected** route this
//! module used to carry is gone, and select-twice with it).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{require_edit, AuthUser},
    dto::{ScanRequest, ScanRunListResponse, ScanRunStatusResponse, ScanStartedResponse},
    error::AppError,
    repositories::pipeline_repository::SCAN_STATUS_RUNNING,
    services::theme_scan::ThemeScanError,
    services::theme_scan_run::{
        delete_scenario_scan_run, get_scan_run_status, list_scenario_scan_runs,
    },
    services::theme_scan_start::start_theme_scan,
    state::AppState,
};

/// `POST /cases/:slug/scenarios/:scenario_id/theme-scan` — scan a scenario.
///
/// ## Why this is edit-gated even though a scan writes no candidate facts
///
/// A scan does NOT write `scenario_fact_refs` — the human's ruling is the only
/// path into a scenario's candidate facts. The gate is still correct, for two
/// other reasons: a scan SPENDS REAL LLM BUDGET, and it writes the `scan_runs` /
/// `scan_run_verdicts` audit rows. Both are mutations of the case's record and its
/// cost, so a read-only viewer must not be able to trigger one.
///
/// The `(slug, scenario_id)` pair is case-fenced inside the service. The optional
/// JSON body carries the per-run model picker.
///
/// ## Rust Learning: `Option<Json<T>>` — an OPTIONAL request body
///
/// Axum's `Json<T>` extractor FAILS on an empty body. Wrapping it in `Option`
/// yields `None` when there is no body (or no JSON content type) instead of a
/// 4xx — so an empty `POST` means "scan with the default model". It MUST be the
/// LAST parameter: a body-consuming extractor runs after the non-consuming ones
/// (`AuthUser`, `State`, `Path`).
#[tracing::instrument(skip(state, user, body), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn run_scenario_theme_scan(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
    body: Option<Json<ScanRequest>>,
) -> Result<Json<ScanStartedResponse>, AppError> {
    require_edit(&user)?;
    // No body → the neutral default request (default model).
    let req = body.map(|Json(b)| b).unwrap_or_default();
    tracing::info!(
        "{} POST /cases/{}/scenarios/{}/theme-scan (model={:?})",
        user.username,
        slug,
        scenario_id,
        req.model_id,
    );

    // Parse the path id up front so a malformed id is a clean 400, never a
    // failed DB lookup masquerading as "not found".
    let id = Uuid::parse_str(&scenario_id).map_err(|_| AppError::BadRequest {
        message: "scenario_id must be a valid UUID".to_string(),
        details: json!({ "field": "scenario_id" }),
    })?;

    // The scan runs in the background: this returns as soon as the `running` row
    // is recorded, so the browser → Traefik → Authentik path never waits minutes.
    let started = start_theme_scan(&state, &slug, id, req.model_id)
        .await
        .map_err(map_scan_error)?;
    Ok(Json(ScanStartedResponse {
        run_id: started.run_id,
        status: SCAN_STATUS_RUNNING.to_string(),
        candidates_total: started.candidates_total,
    }))
}

/// `GET /cases/:slug/scenarios/:scenario_id/scan-runs/:run_id` — poll a run.
///
/// Edit-gated (same as the POST — it reads an edit-gated resource; ruling 3) and
/// case-fenced inside the service. Returns the live progress while `running` and
/// the full summary once `completed`.
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id, run_id = %run_id))]
pub async fn get_scenario_scan_run(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id, run_id)): Path<(String, String, String)>,
) -> Result<Json<ScanRunStatusResponse>, AppError> {
    require_edit(&user)?;

    // Both path ids parse up front so a malformed id is a clean 400, not a "not
    // found" masquerade.
    let scenario_uuid = Uuid::parse_str(&scenario_id).map_err(|_| AppError::BadRequest {
        message: "scenario_id must be a valid UUID".to_string(),
        details: json!({ "field": "scenario_id" }),
    })?;
    let run_uuid = Uuid::parse_str(&run_id).map_err(|_| AppError::BadRequest {
        message: "run_id must be a valid UUID".to_string(),
        details: json!({ "field": "run_id" }),
    })?;

    let status = get_scan_run_status(&state, &slug, scenario_uuid, run_uuid)
        .await
        .map_err(map_scan_error)?;
    Ok(Json(status))
}

/// `GET /cases/:slug/scenarios/:scenario_id/scan-runs` — the scenario's run
/// history, newest first.
///
/// Retrieval-only: reads the already-persisted `scan_runs` headers (no verdicts,
/// no summary — those are fetched per-run via the `:run_id` endpoint). Edit-gated
/// and case-fenced identically to the `:run_id` poll (same `require_edit`, same
/// `load_scenario_fenced` inside the service), so a caller cannot list another
/// case's runs.
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn list_scenario_scan_runs_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<ScanRunListResponse>, AppError> {
    require_edit(&user)?;

    // Parse the path id up front so a malformed id is a clean 400, never a failed
    // DB lookup masquerading as an empty history.
    let scenario_uuid = Uuid::parse_str(&scenario_id).map_err(|_| AppError::BadRequest {
        message: "scenario_id must be a valid UUID".to_string(),
        details: json!({ "field": "scenario_id" }),
    })?;

    let runs = list_scenario_scan_runs(&state, &slug, scenario_uuid)
        .await
        .map_err(map_scan_error)?;
    Ok(Json(runs))
}

/// `DELETE /cases/:slug/scenarios/:scenario_id/scan-runs/:run_id` — delete a run.
///
/// Edit-gated (`require_edit`) and case-fenced identically to
/// [`get_scenario_scan_run`] — the delete's `scenario_id` scope is the second
/// fence (see [`delete_scenario_scan_run`]). Success is `204 No Content` (there is
/// no body to return); an unknown run — or a run that belongs to a different
/// scenario — is [`ThemeScanError::ScanRunNotFound`] → 404. Named
/// `_handler` to avoid colliding with the imported service fn of the same base
/// name (mirrors [`list_scenario_scan_runs_handler`]).
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id, run_id = %run_id))]
pub async fn delete_scenario_scan_run_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id, run_id)): Path<(String, String, String)>,
) -> Result<StatusCode, AppError> {
    require_edit(&user)?;

    // Both path ids parse up front so a malformed id is a clean 400, not a "not
    // found" masquerade (identical to the GET poll).
    let scenario_uuid = Uuid::parse_str(&scenario_id).map_err(|_| AppError::BadRequest {
        message: "scenario_id must be a valid UUID".to_string(),
        details: json!({ "field": "scenario_id" }),
    })?;
    let run_uuid = Uuid::parse_str(&run_id).map_err(|_| AppError::BadRequest {
        message: "run_id must be a valid UUID".to_string(),
        details: json!({ "field": "run_id" }),
    })?;

    delete_scenario_scan_run(&state, &slug, scenario_uuid, run_uuid)
        .await
        .map_err(map_scan_error)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Map a [`ThemeScanError`] onto its HTTP surface.
///
/// The split is deliberate (Standing Rule 1 — a caller can tell *what* went
/// wrong): user-fixable preconditions are 4xx with a `details` hint; a missing
/// dependency is a 503 the operator corrects; everything else is a server-side
/// 500 whose cause chain is logged here rather than returned.
///
/// That last rule has ONE exception, and it is principled rather than
/// convenient: a 500 may stay generic only because the run row carries the real
/// reason for the reader to open. The failures that happen BEFORE a run row
/// exists have no such second surface, so they keep their message. See the arm
/// itself for which variants those are and why.
fn map_scan_error(err: ThemeScanError) -> AppError {
    // Compute the display message once. For most server-side variants it is only
    // logged; for the pre-row ones it is also what the caller receives.
    let message = err.to_string();
    match err {
        ThemeScanError::ScenarioNotFound { .. } | ThemeScanError::ScanRunNotFound { .. } => {
            AppError::NotFound { message }
        }
        ThemeScanError::EmptyAttackMeaning { .. } => AppError::BadRequest {
            message,
            details: json!({ "precondition": "attack_meaning" }),
        },
        ThemeScanError::SubjectUnresolvable { .. } => AppError::BadRequest {
            message,
            details: json!({ "precondition": "subject" }),
        },
        // The run is part of the record — rulings cite it as the scan that
        // proposed them. A 409 (not a 403 or a 400): the request is well-formed and
        // the caller is permitted — it conflicts with the current STATE of the
        // resource. Nothing the caller can fix by retrying or rephrasing, which is
        // why the message explains that the provenance is kept on purpose rather
        // than implying a transient fault.
        ThemeScanError::ScanRunCited { .. } => AppError::Conflict {
            message,
            details: json!({ "reason": "run_cited" }),
        },
        // Bad model CHOICE (unknown/inactive, un-satisfiable params, or an
        // un-buildable row like a vLLM model with no endpoint): the operator
        // fixes it by picking a valid model — 400 with the reason.
        ThemeScanError::ModelNotAvailable { .. }
        | ThemeScanError::ParamsInvalid { .. }
        | ThemeScanError::ProviderBuildFailed { .. } => AppError::BadRequest {
            message,
            details: json!({ "precondition": "model" }),
        },
        // HARD GATE refusals: the selected vLLM endpoint is down or serving the
        // wrong model — a dependency problem the operator corrects. 503.
        ThemeScanError::VllmUnreachable { .. } | ThemeScanError::VllmModelMismatch { .. } => {
            AppError::ServiceUnavailable { message }
        }
        // A missing judging prompt is the SAME shape as the gate refusals: a
        // deployment dependency the operator corrects (deploy the file, or point
        // the `theme_scan_prompt_file` row at one that exists), not a bug in the
        // request and not an opaque
        // server fault. It was previously folded into the 500 below, which threw
        // away the one thing that makes it fixable — the path. 503 with the full
        // message, which names the path and the recovery action.
        ThemeScanError::PromptFileMissing { .. } => {
            tracing::error!(error = %message, "theme scan: judging prompt unreadable");
            AppError::ServiceUnavailable { message }
        }
        // The pre-stub server-side failures: still 500 (nothing about the request
        // is wrong), but they KEEP their message.
        //
        // The generic-500 policy below rests on an assumption these two break.
        // It is safe to answer "theme scan failed" only because the run row
        // carries the real reason — the operator opens Run History and reads it.
        // Since the 400 split these fail BEFORE the row exists, so a generic
        // message leaves nothing anywhere: no row to open, and a toast that names
        // neither the scenario, the model, nor what to do. Their `#[error]` strings
        // were written to carry a recovery action precisely because this is their
        // only surface; discarding them here would make that a lie.
        //
        // Two, not three: `SubjectResolveFailed` was retired on 2026-08-07 with
        // the case-default fallback that was its only cause.
        ThemeScanError::DefinitionInvalid { .. } | ThemeScanError::ModelLookupFailed { .. } => {
            tracing::error!(error = %message, "theme scan: failed before any run was recorded");
            AppError::Internal { message }
        }
        // Everything else server-side: DB and graph failures that happen AFTER the
        // run row exists. Log the full typed error (with its source) and return a
        // generic 500 — the row is where the detail lives.
        other => {
            tracing::error!(error = %other, "theme scan failed (server-side)");
            AppError::Internal {
                message: "theme scan failed".to_string(),
            }
        }
    }
}

#[cfg(test)]
#[path = "scenario_theme_scan_tests.rs"]
mod tests;
