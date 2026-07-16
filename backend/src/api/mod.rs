use axum::{
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    routing::{delete, get, patch, post, put},
    Json, Router,
};

use crate::auth::{me_handler, AuthUser, MeResponse};
use crate::bias::handlers as bias_handlers;
use crate::repositories::pipeline_repository::users as known_users;
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
pub mod analysis;
pub mod ask;
pub mod case;
pub mod case_header;
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
pub mod graph;
pub mod harms;
pub mod import;
pub mod logout;
pub mod persons;
pub mod pipeline;
pub mod proof_matrix;
pub mod proof_review;
pub mod qa;
pub mod queries;
pub mod scenario_facts;
pub mod scenario_gather;
pub mod scenario_theme_scan;
pub mod scenarios;
pub mod schema;
pub mod search;
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
/// This top-level function is a table of contents: it `.merge()`s the
/// route-group functions below. Each group is a small, focused unit (kept
/// under the 50-line function limit); `.merge()` is order-independent here
/// because every route path is distinct, so there is no overlap precedence
/// to worry about.
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(session_routes())
        .merge(case_routes())
        .merge(scenario_routes())
        .merge(claim_routes())
        .merge(document_routes())
        .merge(entity_routes())
        .merge(decomposition_routes())
        .merge(query_routes())
        .merge(admin_document_routes())
        .merge(admin_ops_routes())
        .merge(interaction_routes())
}

/// Session / identity routes: who-am-I, known users, logout.
fn session_routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(me_with_tracking))
        .route("/users", get(pipeline::users::list_users_handler))
        .route("/logout", get(logout::logout))
}

/// Case-level reads: the analysis dashboard, the legacy case summary, and the
/// slug-scoped case header + causes-of-action endpoints.
fn case_routes() -> Router<AppState> {
    Router::new()
        .route("/analysis", get(analysis::get_analysis))
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

/// Scenario authoring + curation routes (the `/cases/:slug/scenarios/...`
/// cluster). Split out of `case_routes` as its own group so each route-group
/// function stays under the function-size limit and the scenario surface reads
/// as one unit. Merged independently in `router()`; paths are distinct from the
/// other groups', so merge order does not matter.
fn scenario_routes() -> Router<AppState> {
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
        // Scenario fact curation (Phase A): save / list / remove the graph facts
        // a human curates onto a scenario. Reads are open (Option<AuthUser>);
        // the write routes enforce `require_edit` inside their handlers.
        .route(
            "/cases/:slug/scenarios/:scenario_id/facts",
            get(scenario_facts::list_scenario_facts).post(scenario_facts::add_scenario_fact),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id/facts/:graph_node_id",
            delete(scenario_facts::remove_scenario_fact),
        )
        // Candidate-workbench ruling (Phase 1a.3): include / drop / un-drop one
        // candidate via a typed action enum. Edit-gated inside the handler. A
        // static `action` child under the `:graph_node_id` param — beside the
        // `/facts/gather` static sibling that matchit 0.7.3 already accepts.
        .route(
            "/cases/:slug/scenarios/:scenario_id/facts/:graph_node_id/action",
            post(scenario_facts::apply_fact_action),
        )
        // Candidate-workbench gather (Phase 1a.2): read-only pool of every
        // Evidence node ABOUT the scenario's subject, each tagged with its
        // derived workbench status. Open read (Option<AuthUser>), like the
        // sibling facts list.
        .route(
            "/cases/:slug/scenarios/:scenario_id/facts/gather",
            get(scenario_gather::gather_scenario_candidates),
        )
        // Theme Scan (D2b): LLM-judge every candidate quote about the scenario's
        // subject and persist the relevant verdicts as confirmed=false
        // suggestions. Edit-gated inside the handler (writes + real LLM spend).
        .route(
            "/cases/:slug/scenarios/:scenario_id/theme-scan",
            post(scenario_theme_scan::run_scenario_theme_scan),
        )
        // Poll one background scan run: live progress while running, full summary
        // when completed. Edit-gated + case-fenced inside the handler.
        .route(
            "/cases/:slug/scenarios/:scenario_id/scan-runs/:run_id",
            get(scenario_theme_scan::get_scenario_scan_run),
        )
        // List a scenario's scan-run HISTORY (headers only, newest first) so the
        // panel hydrates from the DB and survives navigation. Retrieval-only,
        // edit-gated + case-fenced inside the handler.
        .route(
            "/cases/:slug/scenarios/:scenario_id/scan-runs",
            get(scenario_theme_scan::list_scenario_scan_runs_handler),
        )
}

/// Claim CRUD plus the motion-claims read.
fn claim_routes() -> Router<AppState> {
    Router::new()
        .route("/claims", get(claims::list_claims))
        .route("/claims/:id", get(claims::get_claim))
        .route("/claims", post(claims::create_claim))
        .route("/claims/:id", put(claims::update_claim))
        .route("/motion-claims", get(claims::list_motion_claims))
}

/// Document CRUD + file download, import validation, and the schema read.
fn document_routes() -> Router<AppState> {
    Router::new()
        .route("/documents", get(documents::list_documents))
        .route("/documents", post(documents::create_document))
        .route("/documents/:id", get(documents::get_document))
        .route("/documents/:id", put(documents::update_document))
        .route("/documents/:id/file", get(documents::get_document_file))
        .route("/import/validate", post(import::validate_import))
        .route("/schema", get(schema::get_schema))
}

/// Graph-entity reads: persons, allegations, evidence, harms, contradictions,
/// and the legal-proof graph.
fn entity_routes() -> Router<AppState> {
    Router::new()
        .route("/persons", get(persons::list_persons))
        .route("/persons/:id/detail", get(persons::get_person_detail))
        .route("/allegations", get(allegations::list_allegations))
        .route(
            "/allegations/:id/evidence-chain",
            get(evidence_chain::get_evidence_chain),
        )
        .route("/evidence", get(evidence::list_evidence))
        .route("/harms", get(harms::list_harms))
        .route("/contradictions", get(contradictions::list_contradictions))
        .route("/graph/legal-proof", get(graph::get_legal_proof_graph))
}

/// Decomposition intelligence: characterizations, per-allegation detail,
/// and rebuttals.
fn decomposition_routes() -> Router<AppState> {
    Router::new()
        .route("/decomposition", get(decomposition::list_decomposition))
        .route(
            "/allegations/:id/detail",
            get(decomposition::get_allegation_detail),
        )
        .route("/rebuttals", get(decomposition::list_rebuttals))
}

/// Saved-query list and run.
fn query_routes() -> Router<AppState> {
    Router::new()
        .route("/queries", get(queries::list_queries))
        .route("/queries/:id/run", get(queries::run_query))
}

/// Admin document-lifecycle routes: embedding, registration, reindex, upload,
/// and per-document evidence/extract/verify/flag/ground-pages operations.
fn admin_document_routes() -> Router<AppState> {
    Router::new()
        .route("/admin/embed-all", post(embed::run_embed_all))
        .route(
            "/admin/documents",
            get(admin_documents::list_documents).post(admin_documents::register_document),
        )
        .route("/admin/reindex", post(admin_reindex::trigger_reindex))
        // Raise axum's 2 MB default body limit so PDF uploads up to
        // the handler's MAX_FILE_SIZE ceiling reach the handler. Scoped
        // to this route only — other admin endpoints keep the tighter
        // default as a safety net against runaway bodies.
        .route(
            "/admin/upload",
            post(admin_upload::upload_file).layer(DefaultBodyLimit::max(pipeline::MAX_FILE_SIZE)),
        )
        .route(
            "/admin/documents/:id/evidence",
            get(admin_document_evidence::get_document_evidence),
        )
        .route(
            "/admin/documents/:id/extracts",
            get(admin_document_extracts::get_document_extracts),
        )
        .route(
            "/admin/documents/:id/evidence/:eid/verify",
            post(admin_verify::verify_evidence),
        )
        .route(
            "/admin/documents/:id/evidence/:eid/flag",
            post(admin_flag::flag_evidence),
        )
        .route(
            "/admin/documents/:id/ground-pages",
            post(admin_page_ground::ground_pages),
        )
}

/// Admin operational routes: evidence import, QA-entry admin, audit health,
/// status, and the nested pipeline admin router.
fn admin_ops_routes() -> Router<AppState> {
    Router::new()
        .route("/admin/evidence", post(admin_evidence::import_evidence))
        .route(
            "/admin/qa-entries",
            get(admin_qa::list_all_entries).delete(admin_qa::bulk_delete_entries),
        )
        .route("/admin/audit/health", get(admin_audit_health::audit_health))
        .route("/admin/status", get(admin_status::get_status))
        .nest("/admin/pipeline", pipeline::router())
}

/// Interactive / RAG routes: Bias Explorer reads, semantic search, ask,
/// chat models, and Q&A history + rating.
///
/// Bias Explorer routes live in `crate::bias::handlers` (the bias module owns
/// its own DTOs, repository, and handlers as a self-contained feature).
fn interaction_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/bias/available-filters",
            get(bias_handlers::get_available_filters),
        )
        .route("/bias/query", post(bias_handlers::post_bias_query))
        .route("/search", post(search::semantic_search))
        .route("/ask", post(ask::ask_the_case))
        .route("/chat/models", get(chat_models::list_chat_models))
        // Scan/benchmark model picker — active AND scan_eligible only, so retired
        // (but extraction-active) models stay out of the picker (ruling A).
        .route("/scan/models", get(chat_models::list_scan_models))
        .route("/qa-history", get(qa::get_qa_history))
        .route("/qa/:id", get(qa::get_qa_entry).delete(qa::delete_qa_entry))
        .route("/qa/:id/rate", patch(qa::rate_qa_entry))
}

/// Wrapper around `me_handler` that also records the user in `known_users`.
///
/// The upsert is fire-and-forget: it runs in a background task so it never
/// slows down or fails the `/api/me` response. This is the simplest way to
/// passively track users without adding middleware complexity.
///
/// ## Rust Learning: tokio::spawn for fire-and-forget
///
/// `tokio::spawn` launches a new async task on the runtime. The spawned
/// future runs independently — we don't `.await` the JoinHandle, so the
/// response returns immediately.
async fn me_with_tracking(user: AuthUser, State(state): State<AppState>) -> Json<MeResponse> {
    // Clone the values the background task needs before we move `user`.
    let pool = state.pipeline_pool.clone();
    let username = user.username.clone();
    let display_name = user.display_name.clone();
    let email = user.email.clone();

    tokio::spawn(async move {
        known_users::upsert_known_user(&pool, &username, &display_name, &email)
            .await
            // best-effort: passive user-tracking upsert in a detached task; a DB failure must never fail or delay the /api/me response.
            .ok();
    });

    me_handler(user).await
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
