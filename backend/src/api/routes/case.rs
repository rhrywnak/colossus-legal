//! Case-level reads: the legacy case summary and the slug-scoped case header +
//! causes-of-action, element, proof and trial-prep endpoints.

use axum::{
    routing::{get, patch},
    Router,
};

use crate::api::{
    case, case_header, case_summary, causes_of_action, element_detail, proof_matrix, proof_review,
    trial_prep,
};
use crate::state::AppState;

/// Case-level reads: the legacy case summary and the slug-scoped case header +
/// causes-of-action endpoints.
///
/// `GET /analysis` was removed with the Evidence explorer (nav cleanup Part 2):
/// its only callers were that page and its parts, both retired.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/case", get(case::get_case))
        .route("/case-summary", get(case_summary::get_case_summary))
        .route("/cases/:slug", get(case_header::get_case_by_slug))
        .route(
            "/cases/:slug/causes-of-action",
            get(causes_of_action::get_causes_of_action),
        )
        .route(
            "/cases/:slug/elements/:element_id/detail",
            get(element_detail::get_element_detail),
        )
        .route(
            "/cases/:slug/elements/:element_id/notes",
            patch(element_detail::patch_element_notes),
        )
        .route(
            "/cases/:slug/proof-matrix/rollup",
            get(proof_matrix::get_proof_matrix_rollup),
        )
        .route(
            "/cases/:slug/proof-review",
            get(proof_review::get_proof_review),
        )
        .route(
            "/cases/:slug/trial-prep/dashboard",
            get(trial_prep::get_trial_prep_dashboard),
        )
        .route(
            "/cases/:slug/trial-prep/scenarios/:scenario_id",
            get(trial_prep::get_trial_prep_scenario_detail),
        )
}
