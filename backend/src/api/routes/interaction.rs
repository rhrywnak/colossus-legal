//! Interactive / RAG routes: Bias Explorer reads, semantic search, ask,
//! chat models, and Q&A history + rating.

use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::api::{ask, chat_models, qa, search};
use crate::bias::handlers as bias_handlers;
use crate::state::AppState;

/// Interactive / RAG routes: Bias Explorer reads, semantic search, ask,
/// chat models, and Q&A history + rating.
///
/// The bias module keeps its FILTER half and loses its query half (nav cleanup
/// Part 2). `POST /bias/query` served the Bias Explorer page and nothing else,
/// and the page is removed. `GET /bias/available-filters` STAYS: it is read by
/// `ScenarioCreateForm` and `ScenarioIdentityModal` so the people they offer
/// match the filter's — a live scenario surface, not an explorer remnant.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/bias/available-filters",
            get(bias_handlers::get_available_filters),
        )
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
