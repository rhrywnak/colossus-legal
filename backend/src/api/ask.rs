//! "Ask the Case" endpoint — powered by colossus-rag's RagPipeline.
//!
//! `POST /ask` delegates to the shared RAG pipeline which orchestrates:
//! 1. **Route** — classify the question via RuleBasedRouter
//! 2. **Search** — embed + search Qdrant via QdrantRetriever
//! 3. **Expand** — traverse Neo4j via Neo4jExpander
//! 4. **Assemble** — format context via LegalAssembler
//! 5. **Synthesize** — call Claude via RigSynthesizer
//!
//! ## What changed from the old Minerva handler
//!
//! The old handler (~250 lines) orchestrated 5 stages manually:
//! embed_question → search_points → expand_context → build_system_prompt → synthesize.
//! Each stage had its own error handling, timing, and data mapping.
//!
//! The new handler (~80 lines) calls `pipeline.ask(question)` and maps the
//! result to the same `AskResponse` JSON shape. The pipeline handles all
//! orchestration, timing, and error propagation internally.
//!
//! ## CRITICAL: Response shape is unchanged
//!
//! The frontend expects `{ question, answer, provider, retrieval_stats: {...} }`.
//! We map `RagResult` → `AskResponse` field by field to preserve this contract.

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::api::embed::ErrorResponse;
use crate::auth::{AuthUser, require_ai};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / Response types (unchanged from old handler)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AskRequest {
    pub question: String,
}

/// The response shape the frontend expects.
///
/// ## Rust Learning: Keeping API contracts stable
///
/// When swapping out internal implementations, the response type is your
/// contract with consumers (frontend, tests, other services). Every field
/// name, type, and nesting level must stay the same. We map from
/// `colossus_rag::RagResult` to this struct to maintain the contract.
#[derive(Debug, Serialize)]
pub struct AskResponse {
    pub question: String,
    pub answer: String,
    pub provider: String,
    pub retrieval_stats: RetrievalStats,
}

#[derive(Debug, Serialize)]
pub struct RetrievalStats {
    pub qdrant_hits: usize,
    pub graph_nodes_expanded: usize,
    pub context_tokens: usize,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub search_ms: u64,
    pub expand_ms: u64,
    pub synthesis_ms: u64,
    pub total_ms: u64,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

type ApiError = (StatusCode, Json<ErrorResponse>);

/// POST /ask — the full RAG pipeline in one request.
///
/// ## Rust Learning: Thin handler pattern
///
/// This handler does three things:
/// 1. Validate the request (auth, non-empty question, pipeline availability)
/// 2. Call `pipeline.ask(question)` — all orchestration happens inside
/// 3. Map the result to the frontend's expected response shape
///
/// All the complex pipeline logic (routing, embedding, searching, expanding,
/// assembling, synthesizing) lives in colossus-rag. The handler is just glue.
pub async fn ask_the_case(
    user: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<AskRequest>,
) -> Result<Json<AskResponse>, ApiError> {
    require_ai(&user).map_err(|e| error_response(StatusCode::FORBIDDEN, &e.message))?;
    tracing::info!("{} POST /ask", user.username);

    let question = req.question.trim().to_string();
    if question.is_empty() {
        return Err(error_response(StatusCode::BAD_REQUEST, "question must not be empty"));
    }

    // Get the pipeline — returns 503 if not configured (no API key).
    let pipeline = state.rag_pipeline.as_ref().ok_or_else(|| {
        error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "Claude API key not configured. Set ANTHROPIC_API_KEY in .env",
        )
    })?;

    // Run the full pipeline: route → search → expand → assemble → synthesize.
    let result = pipeline.ask(&question).await.map_err(|e| {
        tracing::error!("RAG pipeline error: {e}");
        // Map RagError variants to appropriate HTTP status codes.
        let status = match &e {
            colossus_rag::RagError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        error_response(status, &e.to_string())
    })?;

    tracing::info!(
        question = %question,
        strategy = %result.stats.strategy,
        qdrant_hits = result.stats.qdrant_hits,
        graph_nodes = result.stats.graph_nodes_expanded,
        total_ms = result.stats.total_ms,
        "Ask the Case completed"
    );

    // Map RagResult → AskResponse (preserving the frontend's expected shape).
    Ok(Json(AskResponse {
        question,
        answer: result.answer,
        provider: result.stats.model.clone(),
        retrieval_stats: RetrievalStats {
            qdrant_hits: result.stats.qdrant_hits,
            graph_nodes_expanded: result.stats.graph_nodes_expanded,
            context_tokens: result.stats.context_tokens_approx,
            input_tokens: result.stats.input_tokens.unwrap_or(0),
            output_tokens: result.stats.output_tokens.unwrap_or(0),
            search_ms: result.stats.search_ms,
            expand_ms: result.stats.expand_ms,
            synthesis_ms: result.stats.synthesize_ms,
            total_ms: result.stats.total_ms,
        },
    }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a standardized error response tuple.
fn error_response(status: StatusCode, message: &str) -> ApiError {
    (status, Json(ErrorResponse { error: message.to_string() }))
}
