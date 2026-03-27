use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, patch, post, put},
    Router,
};

use crate::auth::me_handler;
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
pub mod case_summary;
pub mod claims;
pub mod contradictions;
pub mod decomposition;
pub mod documents;
pub mod embed;
pub mod evidence;
pub mod evidence_chain;
pub mod graph;
pub mod harms;
pub mod import;
pub mod logout;
pub mod persons;
pub mod pipeline;
pub mod qa;
pub mod queries;
pub mod schema;
pub mod search;

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
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me", get(me_handler))
        .route("/logout", get(logout::logout))
        .route("/analysis", get(analysis::get_analysis))
        .route("/case", get(case::get_case))
        .route("/case-summary", get(case_summary::get_case_summary))
        .route("/claims", get(claims::list_claims))
        .route("/claims/:id", get(claims::get_claim))
        .route("/claims", post(claims::create_claim))
        .route("/claims/:id", put(claims::update_claim))
        .route("/documents", get(documents::list_documents))
        .route("/documents", post(documents::create_document))
        .route("/documents/:id", get(documents::get_document))
        .route("/documents/:id", put(documents::update_document))
        .route("/documents/:id/file", get(documents::get_document_file))
        .route("/import/validate", post(import::validate_import))
        .route("/schema", get(schema::get_schema))
        .route("/persons", get(persons::list_persons))
        .route("/persons/:id/detail", get(persons::get_person_detail))
        .route("/allegations", get(allegations::list_allegations))
        .route(
            "/allegations/:id/evidence-chain",
            get(evidence_chain::get_evidence_chain),
        )
        .route("/evidence", get(evidence::list_evidence))
        .route("/harms", get(harms::list_harms))
        .route("/motion-claims", get(claims::list_motion_claims))
        .route("/contradictions", get(contradictions::list_contradictions))
        .route("/graph/legal-proof", get(graph::get_legal_proof_graph))
        .route("/decomposition", get(decomposition::list_decomposition))
        .route(
            "/allegations/:id/detail",
            get(decomposition::get_allegation_detail),
        )
        .route("/rebuttals", get(decomposition::list_rebuttals))
        .route("/queries", get(queries::list_queries))
        .route("/queries/:id/run", get(queries::run_query))
        .route("/admin/embed-all", post(embed::run_embed_all))
        .route(
            "/admin/documents",
            get(admin_documents::list_documents).post(admin_documents::register_document),
        )
        .route(
            "/admin/evidence",
            post(admin_evidence::import_evidence),
        )
        .route(
            "/admin/reindex",
            post(admin_reindex::trigger_reindex),
        )
        .route(
            "/admin/qa-entries",
            get(admin_qa::list_all_entries).delete(admin_qa::bulk_delete_entries),
        )
        .route("/admin/upload", post(admin_upload::upload_file))
        .route(
            "/admin/audit/health",
            get(admin_audit_health::audit_health),
        )
        .route(
            "/admin/status",
            get(admin_status::get_status),
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
        .route(
            "/admin/pipeline/documents",
            post(pipeline::upload_document),
        )
        .route(
            "/admin/pipeline/documents/:id/extract-text",
            post(pipeline::extract_text),
        )
        .route(
            "/admin/pipeline/documents/:id/extract",
            post(pipeline::extract_handler),
        )
        .route(
            "/admin/pipeline/documents/:id/verify",
            post(pipeline::verify_handler),
        )
        .route(
            "/admin/pipeline/documents/:id/report",
            get(pipeline::report_handler),
        )
        .route("/search", post(search::semantic_search))
        .route("/ask", post(ask::ask_the_case))
        .route("/qa-history", get(qa::get_qa_history))
        .route("/qa/:id", get(qa::get_qa_entry).delete(qa::delete_qa_entry))
        .route("/qa/:id/rate", patch(qa::rate_qa_entry))
}

/// Health check endpoint — served at `/health` (root level, no `/api/` prefix).
///
/// Kept outside the API router because health checks are a standard
/// convention at the root path, and nginx/load balancers expect it there.
pub async fn health_check(State(_state): State<AppState>) -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
}
