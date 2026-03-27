use std::sync::Arc;

use neo4rs::Graph;
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::repositories::audit_repository::AuditRepository;

/// Shared application state injected into every Axum handler.
///
/// ## Rust Learning: `Arc` for shared ownership across handlers
///
/// `RagPipeline` contains `Box<dyn Trait>` fields which are NOT `Clone`.
/// Axum requires handler state to be `Clone` (it clones state for each request).
/// Wrapping in `Arc` gives us shared ownership: all handlers share the same
/// pipeline instance via reference counting, without needing `Clone` on the
/// pipeline itself.
///
/// `Option<Arc<...>>` means the pipeline is absent when the Anthropic API key
/// is not configured — the `/ask` endpoint returns 503, but every other
/// endpoint works normally.
#[derive(Clone)]
pub struct AppState {
    pub graph: Graph,
    pub config: AppConfig,

    /// The RAG pipeline — None if ANTHROPIC_API_KEY is not set.
    /// Shared across all request handlers via Arc (RagPipeline is not Clone).
    pub rag_pipeline: Option<Arc<colossus_rag::RagPipeline>>,

    /// Shared HTTP client with timeouts for all outbound requests.
    /// reqwest::Client uses an internal Arc, so cloning is cheap.
    pub http_client: reqwest::Client,

    /// PostgreSQL connection pool for analytical data (ratings, feedback).
    /// PgPool uses an internal Arc, so cloning is cheap.
    pub pg_pool: PgPool,

    /// PostgreSQL pool for the pipeline v2 database (extraction, review, pipeline state).
    /// Separate from pg_pool which connects to the existing colossus_legal database.
    pub pipeline_pool: PgPool,

    /// Audit log repository for recording admin actions.
    pub audit_repo: AuditRepository,
}
