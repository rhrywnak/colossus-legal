use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, patch, post, put},
    Router,
};

use crate::auth::me_handler;
use crate::state::AppState;

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
pub mod qa;
pub mod queries;
pub mod schema;
pub mod search;

/// Minimal API router.
///
/// We intentionally expose only a health check here. All of the
/// original Codex-generated routes and logic are preserved in the
/// `wip/codex-refactor-2025-11` branch and can be reintroduced later
/// in small, well-structured feature branches.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/me", get(me_handler))
        .route("/api/logout", get(logout::logout))
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
        .route("/search", post(search::semantic_search))
        .route("/ask", post(ask::ask_the_case))
        .route("/api/qa-history", get(qa::get_qa_history))
        .route("/api/qa/:id", get(qa::get_qa_entry))
        .route("/api/qa/:id/rate", patch(qa::rate_qa_entry))
}

async fn health_check(State(_state): State<AppState>) -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
}
