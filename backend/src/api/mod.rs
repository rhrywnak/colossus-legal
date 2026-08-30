use axum::{extract::State, http::StatusCode, Router};

use crate::state::AppState;

pub mod admin_audit_health;
pub mod admin_document_evidence;
pub mod admin_document_evidence_queries;
pub mod admin_document_extracts;
pub mod admin_documents;
pub mod admin_evidence;
pub mod admin_evidence_helpers;
pub mod admin_flag;
pub mod admin_page_ground;
pub mod admin_qa;
pub mod admin_reindex;
pub mod admin_status;
pub mod admin_upload;
pub mod admin_verify;
pub mod allegations;
pub mod ask;
pub mod case;
pub mod case_header;
pub mod case_health;
pub mod case_summary;
pub mod causes_of_action;
pub mod chat_models;
pub mod claims;
pub mod contradictions;
pub mod decomposition;
pub mod documents;
pub mod element_detail;
pub mod embed;
pub mod evidence;
pub mod evidence_chain;
pub mod evidence_links;
pub mod evidence_summary;
pub mod graph;
pub mod harms;
pub mod import;
pub mod logout;
pub mod persons;
pub mod pipeline;
pub mod practice;
pub mod practice_answers;
pub mod practice_editor;
pub mod practice_editor_add;
pub mod practice_fences;
pub mod practice_flag;
pub mod practice_one_page;
pub mod practice_reorder;
pub mod practice_sessions;
pub mod proof_matrix;
pub mod proof_review;
pub mod qa;
pub mod queries;
pub mod rehearsal;
// The route TABLE — one module per group, split out by T1.0 (R-3).
pub mod routes;
pub mod scenario_accusation;
pub mod scenario_accusation_read;
pub mod scenario_augmentation;
pub mod scenario_augmentation_read;
pub mod scenario_cards;
pub mod scenario_fact_curation;
pub mod scenario_fact_curation_reads;
pub mod scenario_facts;
pub mod scenario_facts_mapping;
pub mod scenario_gather;
pub mod scenario_orphans;
pub mod scenario_theme_scan;
pub mod scenarios;
pub mod schema;
pub mod search;
pub mod settings;
pub mod timeline;
pub mod timeline_subsets;
pub mod timeline_write;
pub mod trial_prep;

/// API router — all routes are relative (no `/api/` prefix).
///
/// The `/api/` prefix is applied structurally in `main.rs` via
/// `Router::nest("/api", api::router())`. This means every route
/// defined here automatically gets the `/api/` prefix at runtime.
///
/// ## Rust Learning: Router::nest()
/// Axum's `.nest(prefix, router)` prepends `prefix` to every route
/// in `router`. A route defined as `.route("/documents", ...)` here
/// becomes `/api/documents` in the final app. This is similar to
/// Express.js `app.use('/api', apiRouter)`.
///
/// This top-level function is a table of contents and nothing else, which is
/// what the T1.0 split (R-3, 2026-08-26) left it as: it `.merge()`s route-group
/// functions that all live elsewhere. Two kinds appear below, and the
/// difference is deliberate rather than historical:
///
/// * `routes::<group>::routes()` — the eleven groups from [`routes`], which
///   never had one owning handler module. See that module's header.
/// * `<handler>::routes()` — a module that owns a whole surface owns its own
///   paths (`timeline`, `practice`, `settings`, `case_health`, …). Those were
///   already split and were not touched by T1.0.
///
/// `.merge()` is order-independent here because every route path is distinct,
/// so there is no overlap precedence to worry about — and
/// `router_builds_without_route_conflicts` below is what keeps that true, since
/// axum panics on a duplicate `(path, method)` at construction.
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(routes::session::routes())
        .merge(routes::case::routes())
        .merge(case_health::routes())
        .merge(routes::scenario::routes())
        .merge(scenario_facts::routes())
        .merge(evidence_summary::routes())
        .merge(evidence_links::routes())
        // Task 2.13: weighing and placing a fact already ruled in. Its own group
        // beside the ruling routes — augmentation, never a ruling (§8).
        .merge(scenario_fact_curation::routes())
        .merge(scenario_augmentation::routes())
        // Task 2.11: the accusation, its marked instances and their answers. One
        // module, one fence — the three write concerns share the guards, never a
        // statement (the "one write path" reading recorded 2026-08-06).
        .merge(scenario_accusation::routes())
        .merge(rehearsal::routes())
        .merge(practice::routes())
        .merge(settings::routes())
        .merge(timeline::routes())
        .merge(routes::subset::routes())
        .merge(routes::claim::routes())
        .merge(routes::document::routes())
        .merge(routes::entity::routes())
        .merge(routes::decomposition::routes())
        .merge(routes::query::routes())
        .merge(routes::admin_document::routes())
        .merge(routes::admin_ops::routes())
        .merge(routes::interaction::routes())
}

/// Health check endpoint — served at `/health` (root level, no `/api/` prefix).
///
/// Kept outside the API router because health checks are a standard
/// convention at the root path, and nginx/load balancers expect it there.
pub async fn health_check(State(_state): State<AppState>) -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Building the router exercises axum's route-conflict detection, which
    /// panics on a duplicate `(path, method)`. Neither `cargo build` nor a
    /// route-equivalence diff catches that — only constructing the router
    /// does. This guards the route-group refactor against an accidental
    /// overlap.
    #[test]
    fn router_builds_without_route_conflicts() {
        let _ = router();
    }
}

/// The route-table walk: the identity proof behind the T1.0 split.
#[cfg(test)]
#[path = "route_table_tests.rs"]
mod route_table_tests;
