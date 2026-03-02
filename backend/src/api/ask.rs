//! "Ask the Case" endpoint — the complete Minerva pipeline.
//!
//! `POST /ask` chains five async steps:
//! 1. **Embed** — convert the question to a 768-dim vector via fastembed
//! 2. **Search** — find the top 10 similar nodes in Qdrant
//! 3. **Expand** — traverse 1-2 hops in Neo4j per search hit
//! 4. **Assemble** — build a system prompt with the expanded context
//! 5. **Synthesize** — call Claude API for a cited narrative answer
//!
//! ## Pattern: Full pipeline orchestration
//! Each step is timed with `Instant::now()` / `elapsed()`, and all timing
//! data is returned in `RetrievalStats` so the frontend can display where
//! time was spent. Errors at each step produce specific status codes.

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::api::embed::ErrorResponse;
use crate::auth::{AuthUser, require_ai};
use crate::services::claude_client;
use crate::services::embedding_service::EmbeddingService;
use crate::services::graph_expander::{self, ExpandedContext};
use crate::services::qdrant_service::{self, SearchResult};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AskRequest {
    pub question: String,
}

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

/// Intermediate result from the retrieval pipeline (steps 1-3).
struct RetrievalResult {
    search_results: Vec<SearchResult>,
    expanded: ExpandedContext,
    search_ms: u64,
    expand_ms: u64,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

type ApiError = (StatusCode, Json<ErrorResponse>);

/// POST /ask — the full Minerva pipeline in one request.
///
/// ## Pattern: Optional config with graceful degradation
/// If `ANTHROPIC_API_KEY` is not set, the handler returns 503 immediately.
/// This lets the rest of the app function normally without the key.
pub async fn ask_the_case(
    user: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<AskRequest>,
) -> Result<Json<AskResponse>, ApiError> {
    require_ai(&user).map_err(|e| error_response(StatusCode::FORBIDDEN, &e.message))?;
    tracing::info!("{} POST /ask", user.username);
    let total_start = Instant::now();

    let question = req.question.trim().to_string();
    if question.is_empty() {
        return Err(error_response(StatusCode::BAD_REQUEST, "question must not be empty"));
    }

    let api_key = state.config.anthropic_api_key.as_deref().ok_or_else(|| {
        error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "Claude API key not configured. Set ANTHROPIC_API_KEY in .env",
        )
    })?;

    // Steps 1-3: embed → search → expand
    let retrieval = retrieve_context(&state, &question).await?;

    // Step 4: assemble system prompt
    let system_prompt = build_system_prompt(&retrieval.expanded.formatted_text);

    // Step 5: synthesize via Claude
    let synthesis_start = Instant::now();
    let client = reqwest::Client::new();
    let result = claude_client::synthesize(
        &client, api_key, &state.config.anthropic_model,
        &system_prompt, &question,
    )
    .await
    .map_err(|e| {
        tracing::error!("Claude synthesis failed: {e}");
        error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("synthesis error: {e}"))
    })?;
    let synthesis_ms = synthesis_start.elapsed().as_millis() as u64;
    let total_ms = total_start.elapsed().as_millis() as u64;

    tracing::info!(
        question = %question,
        qdrant_hits = retrieval.search_results.len(),
        graph_nodes = retrieval.expanded.unique_nodes,
        input_tokens = result.input_tokens,
        output_tokens = result.output_tokens,
        total_ms,
        "Ask the Case completed"
    );

    Ok(Json(build_response(
        question, result, &state.config.anthropic_model,
        retrieval, synthesis_ms, total_ms,
    )))
}

// ---------------------------------------------------------------------------
// Pipeline helpers
// ---------------------------------------------------------------------------

/// Assemble the final AskResponse from pipeline results.
fn build_response(
    question: String,
    result: claude_client::SynthesisResult,
    model: &str,
    retrieval: RetrievalResult,
    synthesis_ms: u64,
    total_ms: u64,
) -> AskResponse {
    AskResponse {
        question,
        answer: result.answer,
        provider: model.to_string(),
        retrieval_stats: RetrievalStats {
            qdrant_hits: retrieval.search_results.len(),
            graph_nodes_expanded: retrieval.expanded.unique_nodes,
            context_tokens: retrieval.expanded.approx_tokens,
            input_tokens: result.input_tokens,
            output_tokens: result.output_tokens,
            search_ms: retrieval.search_ms,
            expand_ms: retrieval.expand_ms,
            synthesis_ms,
            total_ms,
        },
    }
}

/// Steps 1-3: embed the question, search Qdrant, expand through Neo4j.
///
/// ## Pattern: Timed pipeline stages
/// Each step is timed with `Instant::now()` / `elapsed()`. The timings
/// are returned alongside the data so the handler can build `RetrievalStats`.
async fn retrieve_context(
    state: &AppState,
    question: &str,
) -> Result<RetrievalResult, ApiError> {
    // 1. EMBED
    let search_start = Instant::now();
    let vector = embed_question(&state.config.fastembed_cache_path, question).await?;

    // 2. SEARCH
    let client = reqwest::Client::new();
    let search_results = qdrant_service::search_points(
        &client, &state.config.qdrant_url, vector, 10, None,
    )
    .await
    .map_err(|e| {
        tracing::error!("Qdrant search failed: {e}");
        error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("search error: {e}"))
    })?;
    let search_ms = search_start.elapsed().as_millis() as u64;

    // 3. EXPAND
    let expand_start = Instant::now();
    let seeds: Vec<(String, String)> = search_results
        .iter()
        .map(|r| (r.node_id.clone(), r.node_type.clone()))
        .collect();

    let expanded = graph_expander::expand_context(&state.graph, seeds, 6000)
        .await
        .map_err(|e| {
            tracing::error!("Graph expansion failed: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("expand error: {e}"))
        })?;
    let expand_ms = expand_start.elapsed().as_millis() as u64;

    Ok(RetrievalResult { search_results, expanded, search_ms, expand_ms })
}

/// Embed the question text via fastembed (runs on blocking thread pool).
async fn embed_question(
    cache_path: &str,
    question: &str,
) -> Result<Vec<f32>, ApiError> {
    let query_text = format!("search_query: {question}");
    let cache_path = cache_path.to_string();

    tokio::task::spawn_blocking(move || {
        let mut service = EmbeddingService::new(&cache_path)
            .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("embedding init: {e}")))?;
        service.embed_one(&query_text)
            .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("embedding error: {e}")))
    })
    .await
    .map_err(|e| {
        tracing::error!("spawn_blocking panicked: {e}");
        error_response(StatusCode::INTERNAL_SERVER_ERROR, "embedding task failed")
    })?
}

/// Build the system prompt with case context from the knowledge graph.
fn build_system_prompt(context: &str) -> String {
    format!(
r#"You are a legal research assistant for the case Marie Awad v. Catholic Family Service and George Phillips (Bay County Circuit Court).

You have been given evidence from the case knowledge graph, including verbatim quotes from sworn testimony, court filings, and documentary evidence. Each piece of evidence includes its source document and page number.

RULES:
1. Answer using ONLY the provided evidence. Do not infer facts not present in the evidence.
2. For every factual claim in your answer, cite the specific evidence ID in parentheses.
3. When evidence items contradict each other, note the contradiction explicitly and identify which party made each statement.
4. If the provided evidence does not contain enough information to answer the question, say so clearly. Do not speculate.
5. Use plain language accessible to a non-lawyer, but maintain legal precision for citations.
6. When describing patterns (e.g., "Phillips repeatedly..."), list each specific instance with its citation.

CONTEXT FROM KNOWLEDGE GRAPH:
{context}"#
    )
}

/// Build a standardized error response tuple.
fn error_response(status: StatusCode, message: &str) -> ApiError {
    (status, Json(ErrorResponse { error: message.to_string() }))
}
