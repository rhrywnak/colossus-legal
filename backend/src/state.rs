use std::collections::HashMap;
use std::sync::Arc;

use colossus_extract::{EmbeddingProvider, LlmProvider};
use neo4rs::Graph;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::Semaphore;

use crate::config::AppConfig;
use crate::pipeline::extraction_engine::ExtractionEngine;
use crate::pipeline::registry::PipelineRegistry;
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

    /// Embedding provider — fastembed or vLLM, selected by EMBEDDING_PROVIDER
    /// env var at startup. Used by handlers that need to query embedding
    /// dimensions (e.g., for Qdrant collection sizing) or invoke embeddings
    /// directly. See `colossus_extract::providers` for the factory.
    ///
    /// The trait object is the single source of truth for the provider's
    /// configuration; handlers should call methods on it rather than carrying
    /// extracted copies of its values elsewhere.
    pub embedding_provider: Arc<dyn EmbeddingProvider>,

    /// Schema metadata loaded at startup from the extraction schema YAML.
    /// Provides entity type and relationship type names to the query layer
    /// and frontend via GET /api/schema.
    pub schema_metadata: SchemaMetadata,

    /// Per-model LLM providers for the Chat endpoint, built once at startup
    /// from the active rows in `llm_models`. Temperature is `None` on every
    /// entry so chat responses have natural variation (distinct from the
    /// pipeline extraction providers, which pin temperature to 0.0 for
    /// deterministic output). Empty when `ANTHROPIC_API_KEY` is unset —
    /// callers must treat a missing key as 503 just like `rag_pipeline`.
    pub chat_providers: HashMap<String, Arc<dyn LlmProvider>>,

    /// Model id the Chat endpoint uses when the request omits `model`.
    /// Hardcoded at startup (see `main.rs`); may not be present in
    /// `chat_providers` if the corresponding `llm_models` row is missing
    /// or inactive — the `/ask` handler surfaces that as a 400.
    pub default_chat_model: String,

    /// Pipeline configuration registry — the authoritative directory
    /// layout and document-type → profile mapping. Loaded once at
    /// startup from `PIPELINE_REGISTRY_FILE` (or the legacy env-var
    /// fallback). Handlers use the registry's path methods
    /// (`registry.profile_path(...)`, etc.) instead of joining
    /// `config.processing_profile_dir` + a filename — same logical
    /// operation, but with the registry the directory layout can
    /// move without recompiling the backend.
    pub registry: Arc<PipelineRegistry>,

    /// Dedicated concurrency cap for Theme Scan LLM calls, sized from
    /// `config.theme_scan_concurrency` (default 4). A scan drives its per-quote
    /// verdicts with `buffer_unordered`, each acquiring a permit here, so the
    /// cap holds ACROSS concurrent scans — not just within one. Deliberately
    /// separate from the pipeline's `llm_semaphore` so a scan and document
    /// extraction never starve each other (D2b STEP-1 concurrency decision).
    pub theme_scan_semaphore: Arc<Semaphore>,

    /// The shared Rig extraction engine, used to construct per-run LLM providers
    /// from an `llm_models` row via `pipeline::providers::provider_for_model`.
    ///
    /// ## Rust Learning: sharing ONE engine Arc, not building a second
    ///
    /// This is the SAME `Arc<dyn ExtractionEngine>` held by `AppContext`
    /// (`pipeline::context`) — cloned (a refcount bump), not reconstructed. One
    /// engine means one underlying HTTP/1.1 reqwest client, refcount-shared
    /// across the pipeline's per-document providers AND the Theme Scan's per-run
    /// provider. The Theme Scan (a domain service on `&AppState`) needs it so it
    /// can call `provider_for_model` — whose anthropic branch wraps this engine —
    /// instead of building its own boot-time Anthropic provider (Chunk B rewire).
    pub extraction_engine: Arc<dyn ExtractionEngine>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Schema metadata — loaded once at startup from the extraction schema YAML
// ─────────────────────────────────────────────────────────────────────────────

/// Schema metadata loaded at startup from the extraction schema YAML.
///
/// ## Rust Learning: Separation from ExtractionSchema
///
/// We don't store the full ExtractionSchema (which includes extraction_rules,
/// valid_patterns, etc.). We extract only the metadata the app needs at runtime:
/// entity type names/descriptions and relationship type names/descriptions.
/// This keeps our AppState lean and avoids coupling to colossus-extract internals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetadata {
    /// The document type this schema handles (e.g., "general_legal")
    pub document_type: String,
    /// Entity type names and descriptions from the schema
    pub entity_types: Vec<EntityTypeInfo>,
    /// Relationship type names and descriptions from the schema
    pub relationship_types: Vec<RelationshipTypeInfo>,
}

/// An entity type defined in the extraction schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTypeInfo {
    pub name: String,
    pub description: String,
}

/// A relationship type defined in the extraction schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipTypeInfo {
    pub name: String,
    pub description: String,
}
