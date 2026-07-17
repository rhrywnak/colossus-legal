//! Theme Scan HTTP route (D2b).
//!
//! One `POST` route that runs the LLM judge over every candidate quote about a
//! scenario's subject and persists the relevant verdicts as `confirmed=false`
//! suggestions. The judgment logic lives in `services::theme_scan`; this module
//! is a thin transport shell — extract, authorize, delegate, map the typed
//! service error onto an HTTP status.

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
        delete_scenario_scan_run, get_scan_run_status, list_scenario_scan_runs, start_theme_scan,
    },
    state::AppState,
};

/// `POST /cases/:slug/scenarios/:scenario_id/theme-scan` — scan a scenario.
///
/// Edit-gated (`require_edit`): a scan WRITES suggestions to
/// `scenario_fact_refs` and spends real LLM budget, so it is a mutation, not a
/// read. The `(slug, scenario_id)` pair is case-fenced inside the service.
/// The optional JSON body carries the per-run model picker and the dry-run flag.
///
/// ## Rust Learning: `Option<Json<T>>` — an OPTIONAL request body
///
/// Axum's `Json<T>` extractor FAILS on an empty body. Wrapping it in `Option`
/// yields `None` when there is no body (or no JSON content type) instead of a
/// 4xx — so an empty `POST` preserves the pre-Chunk-B behavior (default model,
/// non-dry-run). It MUST be the LAST parameter: a body-consuming extractor runs
/// after the non-consuming ones (`AuthUser`, `State`, `Path`).
#[tracing::instrument(skip(state, user, body), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn run_scenario_theme_scan(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
    body: Option<Json<ScanRequest>>,
) -> Result<Json<ScanStartedResponse>, AppError> {
    require_edit(&user)?;
    // No body → the neutral default request (default model, dry_run = false).
    let req = body.map(|Json(b)| b).unwrap_or_default();
    tracing::info!(
        "{} POST /cases/{}/scenarios/{}/theme-scan (model={:?}, dry_run={})",
        user.username,
        slug,
        scenario_id,
        req.model_id,
        req.dry_run,
    );

    // Parse the path id up front so a malformed id is a clean 400, never a
    // failed DB lookup masquerading as "not found".
    let id = Uuid::parse_str(&scenario_id).map_err(|_| AppError::BadRequest {
        message: "scenario_id must be a valid UUID".to_string(),
        details: json!({ "field": "scenario_id" }),
    })?;

    // The scan runs in the background: this returns as soon as the `running` row
    // is recorded, so the browser → Traefik → Authentik path never waits minutes.
    let started = start_theme_scan(&state, &slug, id, req.model_id, req.dry_run)
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
/// API key is a 503 the operator corrects; everything else is a server-side 500
/// whose full cause chain is logged here (not leaked to the client).
fn map_scan_error(err: ThemeScanError) -> AppError {
    // Compute the display message once; the cause chain (`#[source]`) is only
    // logged, never returned, for the server-side variants below.
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
        // DB, graph, prompt-file, and definition-parse failures are server-side.
        // Log the full typed error (with its source) and return a generic 500.
        other => {
            tracing::error!(error = %other, "theme scan failed (server-side)");
            AppError::Internal {
                message: "theme scan failed".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    // `map_scan_error` is the one piece of policy in this transport shell: it
    // decides which failures are the client's fault (4xx), which are a missing
    // dependency (503), and which are server bugs (500). Pin each mapping so a
    // future variant added to a wrong arm is caught here, not in production.

    #[test]
    fn not_found_maps_to_404() {
        let e = ThemeScanError::ScenarioNotFound {
            case_slug: "awad".to_string(),
            scenario_id: Uuid::nil(),
        };
        assert!(matches!(map_scan_error(e), AppError::NotFound { .. }));
    }

    #[test]
    fn empty_attack_meaning_maps_to_400() {
        let e = ThemeScanError::EmptyAttackMeaning {
            scenario_id: Uuid::nil(),
        };
        assert!(matches!(map_scan_error(e), AppError::BadRequest { .. }));
    }

    #[test]
    fn subject_unresolvable_maps_to_400() {
        let e = ThemeScanError::SubjectUnresolvable {
            scenario_id: Uuid::nil(),
        };
        assert!(matches!(map_scan_error(e), AppError::BadRequest { .. }));
    }

    #[test]
    fn vllm_gate_refusals_map_to_503() {
        let unreachable = ThemeScanError::VllmUnreachable {
            endpoint: "http://x:8000".to_string(),
            detail: "connection refused".to_string(),
        };
        assert!(matches!(
            map_scan_error(unreachable),
            AppError::ServiceUnavailable { .. }
        ));
        let mismatch = ThemeScanError::VllmModelMismatch {
            endpoint: "http://x:8000".to_string(),
            selected: "qwen-14b".to_string(),
            loaded: "qwen-7b".to_string(),
        };
        assert!(matches!(
            map_scan_error(mismatch),
            AppError::ServiceUnavailable { .. }
        ));
    }

    #[test]
    fn scan_run_write_failed_maps_to_500() {
        let e = ThemeScanError::ScanRunWriteFailed {
            run_id: Uuid::nil(),
            source: crate::repositories::pipeline_repository::PipelineRepoError::Database(
                "boom".to_string(),
            ),
        };
        assert!(matches!(map_scan_error(e), AppError::Internal { .. }));
    }

    #[test]
    fn scan_run_read_failed_maps_to_500() {
        let e = ThemeScanError::ScanRunReadFailed {
            run_id: Uuid::nil(),
            source: crate::repositories::pipeline_repository::PipelineRepoError::Database(
                "boom".to_string(),
            ),
        };
        assert!(matches!(map_scan_error(e), AppError::Internal { .. }));
    }

    #[test]
    fn scan_run_not_found_maps_to_404() {
        let e = ThemeScanError::ScanRunNotFound {
            run_id: Uuid::nil(),
        };
        assert!(matches!(map_scan_error(e), AppError::NotFound { .. }));
    }

    #[test]
    fn scan_run_list_failed_maps_to_500() {
        // A DB failure listing a scenario's history is server-side: a generic 500
        // whose cause is logged, never leaked (same policy as ScanRunReadFailed).
        let e = ThemeScanError::ScanRunListFailed {
            scenario_id: Uuid::nil(),
            source: crate::repositories::pipeline_repository::PipelineRepoError::Database(
                "boom".to_string(),
            ),
        };
        assert!(matches!(map_scan_error(e), AppError::Internal { .. }));
    }

    #[test]
    fn scan_run_delete_failed_maps_to_500() {
        // A DB failure DELETING a run is server-side: a generic 500 whose cause is
        // logged, never leaked (same policy as ScanRunReadFailed / ScanRunListFailed).
        // Distinct from ScanRunNotFound (zero rows deleted), which maps to 404.
        let e = ThemeScanError::ScanRunDeleteFailed {
            run_id: Uuid::nil(),
            source: crate::repositories::pipeline_repository::PipelineRepoError::Database(
                "boom".to_string(),
            ),
        };
        assert!(matches!(map_scan_error(e), AppError::Internal { .. }));
    }

    #[test]
    fn bad_model_choice_maps_to_400() {
        let e = ThemeScanError::ModelNotAvailable {
            model_id: "nope".to_string(),
        };
        assert!(matches!(map_scan_error(e), AppError::BadRequest { .. }));
    }

    #[test]
    fn params_invalid_maps_to_400() {
        let e = ThemeScanError::ParamsInvalid {
            model_id: "qwen-14b".to_string(),
            source: crate::domain::llm_params::LlmConfigError::ClearNotAllowed {
                param: "max_tokens",
            },
        };
        assert!(matches!(map_scan_error(e), AppError::BadRequest { .. }));
    }

    #[test]
    fn provider_build_failed_maps_to_400() {
        let e = ThemeScanError::ProviderBuildFailed {
            model_id: "llama-3-8b".to_string(),
            detail: "has no api_endpoint".to_string(),
        };
        assert!(matches!(map_scan_error(e), AppError::BadRequest { .. }));
    }

    #[test]
    fn server_side_variants_map_to_500() {
        // A representative server-side variant (prompt file unreadable) must be a
        // generic 500, not leak its cause to the client.
        let e = ThemeScanError::PromptFileMissing {
            path: "/x".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "nope"),
        };
        assert!(matches!(map_scan_error(e), AppError::Internal { .. }));
    }
}
