//! Timeline subsets: both path families, one group (T1.3).
//!
//! ## Why the scenario paths are here and not in [`super::scenario`]
//!
//! They are one feature. `/timeline/subsets…` creates and edits a story;
//! `/cases/:slug/scenarios/:id/subsets…` says which scenario carries it. Reading
//! them together is how somebody sees that a subset is authored in one place and
//! pointed at from another — which is the design's whole shape. Splitting them
//! across two files by URL prefix would sort them by an accident of addressing.
//!
//! It also keeps both route-group functions inside the 50-line limit: the
//! scenario group is already at forty, and three more routes would have pushed
//! it over.

use axum::{
    routing::{get, post, put},
    Router,
};

use crate::api::timeline_subsets::{reads, scenario_links, writes};
use crate::state::AppState;

/// Every timeline-subset route.
///
/// ## ⚑ THE READ/WRITE LINE IS VISIBLE HERE
///
/// The three `get` handlers take `Option<AuthUser>` and are open — looking at a
/// story is not privileged, exactly as looking at the chronology is not. Every
/// other handler takes `AuthUser`, so an anonymous request is a 401 before the
/// body runs; `api::timeline_subsets::writes::tests` proves no write handler is
/// ever declared with the optional extractor.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/timeline/subsets",
            get(reads::get_subsets).post(writes::post_subset),
        )
        .route(
            "/timeline/subsets/:id",
            get(reads::get_subset)
                .put(writes::put_subset)
                .delete(writes::delete_subset),
        )
        // The picker's Save. A PUT and not a PATCH because it REPLACES the
        // ordered set: what the body holds is what the subset will hold, which
        // is what PUT means and what PATCH does not.
        .route(
            "/timeline/subsets/:id/events",
            put(writes::put_subset_events),
        )
        .route(
            "/timeline/subsets/:id/undelete",
            post(writes::post_undelete),
        )
        // The scenario half. `subsets` is a new static child under the
        // `:scenario_id` param, beside `facts`, `theme-scan` and `scan-runs`,
        // which matchit 0.7.3 already accepts.
        .route(
            "/cases/:slug/scenarios/:scenario_id/subsets",
            get(scenario_links::get_scenario_subsets).post(scenario_links::post_scenario_subset),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id/subsets/:subset_id",
            axum::routing::delete(scenario_links::delete_scenario_subset),
        )
}
