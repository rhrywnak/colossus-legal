//! "Ask the Case" endpoint — powered by colossus-rag's RagPipeline.
//!
//! `POST /ask` delegates to the shared RAG pipeline which orchestrates:
//! 1. **Route** — classify the question via RuleBasedRouter
//! 2. **Search** — embed + search Qdrant via QdrantRetriever
//! 3. **Expand** — traverse Neo4j via Neo4jExpander
//! 4. **Assemble** — format context via LegalAssembler
//! 5. **Synthesize** — call Claude via RigSynthesizer
//!
//! The handler calls `pipeline.ask(question)` and maps `RagResult` → `AskResponse`,
//! including retrieval details (chunks, strategy) for frontend transparency.

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::api::embed::ErrorResponse;
use crate::auth::{AuthUser, require_ai};
use crate::repositories::qa_repository::{self, CreateQAEntry};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / Response types (unchanged from old handler)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AskRequest {
    pub question: String,
    /// Optional parent QA ID for follow-up questions.
    #[serde(default)]
    pub parent_qa_id: Option<String>,
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
    /// The persisted QAEntry ID (None if persistence failed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qa_id: Option<String>,

    /// The routing strategy chosen by the pipeline (e.g., "Broad", "Focused(document)")
    pub strategy: String,

    /// Detailed breakdown of every retrieved chunk (Qdrant hits + graph expansion)
    pub retrieval_details: Vec<RetrievalDetail>,
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

/// A single retrieved chunk from the RAG pipeline, exposed for debugging
/// and transparency. Shows what evidence was found and how it was sourced.
///
/// ## Rust Learning: Flattening nested data for API consumers
///
/// ContextChunk has nested SourceReference and Vec<RelatedNode>. Rather than
/// exposing the internal colossus-rag types directly (which would couple the
/// API contract to the library's internals), we flatten into a purpose-built
/// DTO with only the fields the frontend needs.
#[derive(Debug, Serialize)]
pub struct RetrievalDetail {
    /// The knowledge graph node ID (e.g., "evidence-phillips-q74")
    pub node_id: String,

    /// The node type (e.g., "Evidence", "ComplaintAllegation", "MotionClaim")
    pub node_type: String,

    /// Human-readable title
    pub title: String,

    /// Cosine similarity score from Qdrant (0.0 for graph-expanded nodes)
    pub score: f32,

    /// How this chunk was sourced: "qdrant" (vector search) or "graph" (expansion)
    pub origin: String,

    /// Source document title, if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_title: Option<String>,

    /// Source document ID, if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,

    /// Page number in source document, if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<u32>,

    /// Truncated verbatim quote preview (max 200 chars)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_preview: Option<String>,

    /// Number of graph relationships attached to this chunk
    pub relationship_count: usize,
}

/// Map a colossus-rag ContextChunk into an API-facing RetrievalDetail.
///
/// ## Rust Learning: Ownership via `into()` pattern
///
/// This function takes ownership of the chunk (not a reference) because we
/// move strings out of it rather than cloning. Since we're done with the
/// chunks after mapping, this avoids unnecessary allocations.
fn chunk_to_detail(chunk: colossus_rag::ContextChunk) -> RetrievalDetail {
    let origin = if chunk.score > 0.0 {
        "qdrant".to_string()
    } else {
        "graph".to_string()
    };

    // Truncate verbatim quote to 200 chars for preview
    let quote_preview = chunk.source.verbatim_quote.map(|q| {
        if q.len() > 200 {
            format!("{}...", &q[..197])
        } else {
            q
        }
    });

    RetrievalDetail {
        node_id: chunk.node_id,
        node_type: chunk.node_type,
        title: chunk.title,
        score: chunk.score,
        origin,
        document_title: chunk.source.document_title,
        document_id: chunk.source.document_id,
        page_number: chunk.source.page_number,
        quote_preview,
        relationship_count: chunk.relationships.len(),
    }
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

    // Build retrieval stats for response and metadata.
    let retrieval_stats = RetrievalStats {
        qdrant_hits: result.stats.qdrant_hits,
        graph_nodes_expanded: result.stats.graph_nodes_expanded,
        context_tokens: result.stats.context_tokens_approx,
        input_tokens: result.stats.input_tokens.unwrap_or(0),
        output_tokens: result.stats.output_tokens.unwrap_or(0),
        search_ms: result.stats.search_ms,
        expand_ms: result.stats.expand_ms,
        synthesis_ms: result.stats.synthesize_ms,
        total_ms: result.stats.total_ms,
    };

    // Map chunks to retrieval details for the response.
    let retrieval_details: Vec<RetrievalDetail> = result
        .chunks
        .into_iter()
        .map(chunk_to_detail)
        .collect();

    let strategy = result.stats.strategy.clone();

    // Persist Q&A entry to Neo4j.
    // Try to persist, log on failure, but always return the answer.
    let metadata = serde_json::json!({
        "qdrant_hits": retrieval_stats.qdrant_hits,
        "graph_nodes_expanded": retrieval_stats.graph_nodes_expanded,
        "context_tokens": retrieval_stats.context_tokens,
        "input_tokens": retrieval_stats.input_tokens,
        "output_tokens": retrieval_stats.output_tokens,
        "search_ms": retrieval_stats.search_ms,
        "expand_ms": retrieval_stats.expand_ms,
        "synthesis_ms": retrieval_stats.synthesis_ms,
        "total_ms": retrieval_stats.total_ms,
        "strategy": &strategy,
        "retrieval_details": &retrieval_details,
    });

    let qa_create = CreateQAEntry {
        scope_type: "case".to_string(),
        scope_id: "awad-v-cfs-2011".to_string(),
        session_id: None,
        question: question.clone(),
        answer: result.answer.clone(),
        asked_by: user.username.clone(),
        model: result.stats.model.clone(),
        parent_qa_id: req.parent_qa_id,
        metadata: Some(metadata),
    };

    let qa_id = qa_repository::create_qa_entry(&state.graph, qa_create)
        .await
        .map_err(|e| {
            tracing::error!("Failed to persist QA entry: {e}");
        })
        .ok()
        .map(|e| e.id);

    // Map RagResult → AskResponse (preserving the frontend's expected shape).
    Ok(Json(AskResponse {
        question,
        answer: result.answer,
        provider: result.stats.model.clone(),
        retrieval_stats,
        qa_id,
        strategy,
        retrieval_details,
    }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a standardized error response tuple.
fn error_response(status: StatusCode, message: &str) -> ApiError {
    (status, Json(ErrorResponse { error: message.to_string() }))
}
