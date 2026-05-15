//! Application context for pipeline step implementations.
//!
//! `AppContext` is the per-job handle that step implementations receive.
//! It holds the database pools, HTTP client, filesystem paths, trait-object
//! LLM and embedding providers, and a semaphore that bounds concurrent LLM
//! API calls across the worker pool.
//!
//! Providers are constructed at startup from environment variables via
//! `colossus_extract::providers::*_from_env()`. This lets colossus-legal
//! swap Anthropic for vLLM (or fastembed for vLLM embeddings) without any
//! Rust code change — just environment variable changes.
//!
//! Per v5_2 Part 7, with a construction-signature deviation documented in
//! the P2-8 CC instruction (struct-of-args instead of 7 positional args).

use std::sync::Arc;

use colossus_extract::{EmbeddingProvider, LlmProvider};
use neo4rs::Graph;
use reqwest::Client;
use sqlx::PgPool;
use tokio::sync::Semaphore;

use crate::pipeline::extraction_engine::ExtractionEngine;
use crate::pipeline::registry::PipelineRegistry;
use crate::pipeline::rig_provider::RigExtractionEngine;

/// Default number of concurrent LLM calls the pipeline worker will make.
///
/// Kept low to avoid rate-limiting from API providers. Configurable via the
/// `PIPELINE_LLM_CONCURRENCY` env var.
const DEFAULT_LLM_CONCURRENCY: usize = 2;

/// Required dependencies for constructing an [`AppContext`].
///
/// Grouping the required fields into a struct eliminates the silent-swap
/// failure mode of a 7-argument positional constructor (five of those args
/// are `String`; any two adjacent strings could be transposed with no
/// type-level detection).
///
/// Callers construct this with named-field syntax, which is self-documenting
/// and compile-checked:
///
/// ```ignore
/// let ctx = AppContext::from_deps_and_env(AppContextDeps {
///     pipeline_pool,
///     graph,
///     qdrant_url,
///     http_client,
///     registry,
///     document_storage_path,
/// })?;
/// ```
pub struct AppContextDeps {
    /// PostgreSQL pool for the `colossus_legal_v2` (pipeline) database.
    pub pipeline_pool: PgPool,

    /// Neo4j driver handle.
    pub graph: Graph,

    /// Qdrant REST URL (e.g. `http://colossus-dev-db1:6333`).
    /// The gRPC URL is derived by port replacement where needed.
    pub qdrant_url: String,

    /// Shared HTTP client for outbound calls (vLLM, Anthropic, etc.).
    /// One pooled client is re-used across all steps.
    pub http_client: Client,

    /// Filesystem root for document storage
    /// (e.g. `/data/documents`).
    pub document_storage_path: String,

    /// Pipeline configuration registry. Replaces the four previously-
    /// independent directory env vars (`PROCESSING_PROFILE_DIR`,
    /// `EXTRACTION_SCHEMA_DIR`, `EXTRACTION_TEMPLATE_DIR`,
    /// `SYSTEM_PROMPT_DIR`). Step code calls
    /// `context.registry.{profile,schema,template,system_prompt}_path(filename)`
    /// to resolve a file's full path; the same registry also owns the
    /// document_type → profile mapping consumed by the upload route.
    pub registry: Arc<PipelineRegistry>,
}

/// Per-job pipeline execution context.
///
/// Step implementations take `&AppContext` and read from its fields.
/// Construction happens once at process startup in `main.rs`:
/// the resulting `AppContext` is wrapped in `Arc` and cloned into the
/// scheduler/worker.
///
/// Provider fields are `Arc<dyn Trait>` rather than concrete types so that
/// the same `AppContext` type can back Anthropic or vLLM deployments
/// without generics leaking into the step-implementation signatures.
pub struct AppContext {
    /// PostgreSQL pool for the pipeline database.
    pub pipeline_pool: PgPool,

    /// Neo4j driver handle.
    pub graph: Graph,

    /// Qdrant REST URL.
    pub qdrant_url: String,

    /// Shared HTTP client.
    pub http_client: Client,

    /// Filesystem root for document storage.
    pub document_storage_path: String,

    /// Pipeline configuration registry — the authoritative source of
    /// directory layout and document-type → profile mappings. Step
    /// implementations use `context.registry.profile_path(filename)`
    /// (and the schema / template / system_prompt variants) instead
    /// of joining the previously-individual directory strings. The
    /// `Arc` wrapping is identical to how providers are shared — one
    /// instance constructed at startup, reference-counted across all
    /// concurrent step executions.
    pub registry: Arc<PipelineRegistry>,

    /// LLM provider (Anthropic or vLLM).
    /// Constructed from `LLM_PROVIDER` env var at startup.
    /// Consumed by the `LlmExtract` step and by the RAG synthesizer/decomposer.
    pub llm_provider: Arc<dyn LlmProvider>,

    /// Extraction engine — the R4 thin adapter over the LLM client.
    ///
    /// Consumed by the Restate-driven workflow handlers (Phase 2
    /// onwards). The legacy `LlmExtract` / `LlmExtractPass2` pipeline
    /// steps continue to use [`llm_provider`](Self::llm_provider)
    /// until Phase 3 removes them.
    ///
    /// Constructed by [`RigExtractionEngine::from_env`], which reads
    /// the same `ANTHROPIC_API_KEY` that the legacy `llm_provider`
    /// path consumes, plus the optional
    /// `EXTRACTION_ENGINE_TIMEOUT_SECS` and
    /// `EXTRACTION_ENGINE_TCP_KEEPALIVE_SECS` tuning knobs introduced
    /// in P1-3.
    pub extraction_engine: Arc<dyn ExtractionEngine>,

    /// Embedding provider (fastembed or vLLM).
    /// Constructed from `EMBEDDING_PROVIDER` env var at startup.
    /// Consumed by the `Index` step and the RAG `QdrantRetriever`/`EmbeddingReranker`.
    pub embedding_provider: Arc<dyn EmbeddingProvider>,

    /// Global LLM semaphore — limits concurrent LLM API calls across ALL jobs.
    /// Prevents rate-limit collisions between concurrently-processing documents.
    /// Configured via `PIPELINE_LLM_CONCURRENCY`, default `2`.
    pub llm_semaphore: Arc<Semaphore>,
}

impl AppContext {
    /// Construct an `AppContext` from [`AppContextDeps`] plus environment
    /// variables.
    ///
    /// The `AppContextDeps` fields are the things `main.rs` builds from its
    /// config (pools, paths). The env-var-derived fields (providers and the
    /// concurrency semaphore) are constructed here.
    ///
    /// # Errors
    ///
    /// Returns a descriptive error string if:
    /// - `LLM_PROVIDER` is unset, or is set to an unsupported value, or its
    ///   required companion vars are missing (see
    ///   `colossus_extract::providers::llm_provider_from_env`).
    /// - `ANTHROPIC_API_KEY` is unset — required by the new
    ///   [`RigExtractionEngine`] that backs
    ///   [`extraction_engine`](Self::extraction_engine), regardless of
    ///   which `LLM_PROVIDER` is selected for the legacy
    ///   [`llm_provider`](Self::llm_provider). (In practice the legacy
    ///   Anthropic path needs the same key, so this is rarely a new
    ///   failure mode; a vLLM-only deployment is currently the one
    ///   case where the engine refuses to build even though
    ///   `llm_provider` would have succeeded.)
    /// - `EMBEDDING_PROVIDER` is unset, or is set to an unsupported value, or
    ///   its required companion vars are missing (see
    ///   `colossus_extract::providers::embedding_provider_from_env`).
    pub fn from_deps_and_env(deps: AppContextDeps) -> Result<Self, String> {
        let llm_provider = colossus_extract::providers::llm_provider_from_env()
            .map_err(|e| format!("Failed to build LLM provider: {e}"))?;

        // Construct the new Rig-backed extraction engine alongside the
        // legacy `llm_provider`. Both read `ANTHROPIC_API_KEY` from
        // the environment; the legacy provider speaks reqwest 0.12
        // directly while the engine speaks reqwest 0.13 through Rig's
        // `HttpClientExt` (see backend/src/pipeline/rig_provider.rs
        // module doc for why the two HTTP-client versions coexist).
        //
        // The engine is unused until Phase 2 wires it into the
        // Restate workflow handlers; constructing it here at startup
        // means a misconfiguration fails the boot rather than the
        // first inbound job.
        let extraction_engine: Arc<dyn ExtractionEngine> = Arc::new(
            RigExtractionEngine::from_env()
                .map_err(|e| format!("Failed to build extraction engine: {e}"))?,
        );

        let embedding_provider = colossus_extract::providers::embedding_provider_from_env()
            .map_err(|e| format!("Failed to build embedding provider: {e}"))?;

        let llm_concurrency: usize = std::env::var("PIPELINE_LLM_CONCURRENCY")
            .ok() // best-effort: VarError (env var absent) → fall back to DEFAULT_LLM_CONCURRENCY
            .and_then(|v| v.parse().ok()) // best-effort: ParseIntError (non-numeric value) → fall back to DEFAULT_LLM_CONCURRENCY
            .unwrap_or(DEFAULT_LLM_CONCURRENCY);

        Ok(Self {
            pipeline_pool: deps.pipeline_pool,
            graph: deps.graph,
            qdrant_url: deps.qdrant_url,
            http_client: deps.http_client,
            document_storage_path: deps.document_storage_path,
            registry: deps.registry,
            llm_provider,
            extraction_engine,
            embedding_provider,
            llm_semaphore: Arc::new(Semaphore::new(llm_concurrency)),
        })
    }
}
