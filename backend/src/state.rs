use std::sync::Arc;

use neo4rs::Graph;
use serde::{Deserialize, Serialize};
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

    /// Schema metadata loaded at startup from the extraction schema YAML.
    /// Provides entity type and relationship type names to the query layer
    /// and frontend via GET /api/schema.
    pub schema_metadata: SchemaMetadata,
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
