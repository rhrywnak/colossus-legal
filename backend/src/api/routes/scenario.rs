//! Scenario authoring + curation routes (the `/cases/:slug/scenarios/...`
//! cluster).

use axum::{
    routing::{get, post},
    Router,
};

use crate::api::{scenario_theme_scan, scenarios};
use crate::state::AppState;

/// Scenario authoring + curation routes (the `/cases/:slug/scenarios/...`
/// cluster). Split out of `case_routes` as its own group so each route-group
/// function stays under the function-size limit and the scenario surface reads
/// as one unit. Merged independently in `router()`; paths are distinct from the
/// other groups', so merge order does not matter.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/cases/:slug/scenarios",
            get(scenarios::list_scenarios).post(scenarios::create_scenario),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id",
            get(scenarios::get_scenario_by_id)
                .put(scenarios::update_scenario)
                .delete(scenarios::delete_scenario),
        )
        // Theme Scan (D2b): LLM-judge every candidate quote about the scenario's
        // subject and persist the relevant verdicts as confirmed=false
        // suggestions. Edit-gated inside the handler (writes + real LLM spend).
        .route(
            "/cases/:slug/scenarios/:scenario_id/theme-scan",
            post(scenario_theme_scan::run_scenario_theme_scan),
        )
        // Poll one background scan run: live progress while running, full summary
        // when completed. DELETE removes the run (and its verdicts, which cascade).
        // Both edit-gated + case-fenced inside the handler.
        .route(
            "/cases/:slug/scenarios/:scenario_id/scan-runs/:run_id",
            get(scenario_theme_scan::get_scenario_scan_run)
                .delete(scenario_theme_scan::delete_scenario_scan_run_handler),
        )
        // There is no merge route (2026-08-08). A completed run's admitted
        // verdicts reach the queue as a READ-TIME PROJECTION served by
        // `…/facts/cards`, and the human's ruling is the only write — so the
        // second selection this route used to require does not exist to make.
        // List a scenario's scan-run HISTORY (headers only, newest first) so the
        // panel hydrates from the DB and survives navigation. Retrieval-only,
        // edit-gated + case-fenced inside the handler.
        .route(
            "/cases/:slug/scenarios/:scenario_id/scan-runs",
            get(scenario_theme_scan::list_scenario_scan_runs_handler),
        )
}
