//! Graph-entity reads: persons, allegations, evidence, harms, contradictions,
//! and the legal-proof graph.

use axum::{routing::get, Router};

use crate::api::{allegations, contradictions, evidence, evidence_chain, graph, harms, persons};
use crate::state::AppState;

/// Graph-entity reads: persons, allegations, evidence, harms, contradictions,
/// and the legal-proof graph.
pub(crate) fn routes() -> Router<AppState> {
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
