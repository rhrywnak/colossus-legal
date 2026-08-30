//! Admin document-lifecycle routes: embedding, registration, reindex, upload,
//! and per-document evidence/extract/verify/flag/ground-pages operations.

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};

use crate::api::{
    admin_document_evidence, admin_document_extracts, admin_documents, admin_flag,
    admin_page_ground, admin_reindex, admin_upload, admin_verify, embed, pipeline,
};
use crate::state::AppState;

/// Admin document-lifecycle routes: embedding, registration, reindex, upload,
/// and per-document evidence/extract/verify/flag/ground-pages operations.
pub(crate) fn routes() -> Router<AppState> {
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
