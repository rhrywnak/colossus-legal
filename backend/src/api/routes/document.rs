//! Document CRUD + file download, import validation, and the schema read.

use axum::{
    routing::{get, post, put},
    Router,
};

use crate::api::{documents, import, schema};
use crate::state::AppState;

/// Document CRUD + file download, import validation, and the schema read.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/documents", get(documents::list_documents))
        .route("/documents", post(documents::create_document))
        .route("/documents/:id", get(documents::get_document))
        .route("/documents/:id", put(documents::update_document))
        .route("/documents/:id/file", get(documents::get_document_file))
        .route("/import/validate", post(import::validate_import))
        .route("/schema", get(schema::get_schema))
}
