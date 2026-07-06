//! Theme Scan HTTP route (D2b).
//!
//! One `POST` route that runs the LLM judge over every candidate quote about a
//! scenario's subject and persists the relevant verdicts as `confirmed=false`
//! suggestions. The judgment logic lives in `services::theme_scan`; this module
//! is a thin transport shell — extract, authorize, delegate, map the typed
//! service error onto an HTTP status.

use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{require_edit, AuthUser},
    dto::ThemeScanSummary,
    error::AppError,
    services::theme_scan::{run_theme_scan, ThemeScanError},
    state::AppState,
};

/// `POST /cases/:slug/scenarios/:scenario_id/theme-scan` — scan a scenario.
///
/// Edit-gated (`require_edit`): a scan WRITES suggestions to
/// `scenario_fact_refs` and spends real LLM budget, so it is a mutation, not a
/// read. The `(slug, scenario_id)` pair is case-fenced inside the service.
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn run_scenario_theme_scan(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<ThemeScanSummary>, AppError> {
    require_edit(&user)?;
    tracing::info!(
        "{} POST /cases/{}/scenarios/{}/theme-scan",
        user.username,
        slug,
        scenario_id
    );

    // Parse the path id up front so a malformed id is a clean 400, never a
    // failed DB lookup masquerading as "not found".
    let id = Uuid::parse_str(&scenario_id).map_err(|_| AppError::BadRequest {
        message: "scenario_id must be a valid UUID".to_string(),
        details: json!({ "field": "scenario_id" }),
    })?;

    let summary = run_theme_scan(&state, &slug, id)
        .await
        .map_err(map_scan_error)?;
    Ok(Json(summary))
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
        ThemeScanError::ScenarioNotFound { .. } => AppError::NotFound { message },
        ThemeScanError::EmptyAttackMeaning { .. } => AppError::BadRequest {
            message,
            details: json!({ "precondition": "attack_meaning" }),
        },
        ThemeScanError::SubjectUnresolvable { .. } => AppError::BadRequest {
            message,
            details: json!({ "precondition": "subject" }),
        },
        ThemeScanError::ProviderUnavailable => AppError::ServiceUnavailable { message },
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
    fn provider_unavailable_maps_to_503() {
        assert!(matches!(
            map_scan_error(ThemeScanError::ProviderUnavailable),
            AppError::ServiceUnavailable { .. }
        ));
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
