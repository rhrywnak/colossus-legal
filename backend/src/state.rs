use std::collections::HashMap;
use std::sync::Arc;

use crate::services::settings_handle::SettingsHandle;

use colossus_extract::EmbeddingProvider;
use neo4rs::Graph;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

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

    /// The configuration snapshot — every stored tunable, already parsed
    /// (task 1.6, v2 §2b).
    ///
    /// ## Rust Learning: `Arc<T>` for a value that is REPLACED, not mutated
    ///
    /// `AppState` is cloned per request, so a bare `Settings` would be copied
    /// every time and — worse — a copy taken at clone time could never see a
    /// later edit. `Arc` makes the clone a pointer bump, and swapping the whole
    /// `Arc` on write is how "edits take effect on next read" is delivered
    /// without any per-read database work.
    ///
    /// Loaded at boot by `services::settings_store::load_at_boot`, which REFUSES
    /// to start if any parameter is missing or invalid — there are no
    /// compiled-in defaults left to fall back to.
    pub settings: SettingsHandle,

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

    /// The Chat page's model providers, resolved per request against
    /// `llm_models` and built on first use (CC_TASK_MODEL_JOBS_PANEL_v1, B4).
    ///
    /// Replaces the boot-time map and the boot-time copy of `chat_default_model`:
    /// a model added on Admin → Models, or a default saved on Admin → Overview,
    /// now takes effect on the next request with no restart. The default itself
    /// is read from `settings.current().chat_default_model` at each call.
    pub chat_providers: Arc<crate::services::chat_providers_live::ChatProviders>,

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

    /// The STOP handle of every Theme Scan currently judging in this process.
    ///
    /// Keyed by `run_id`; a token is inserted by `services::theme_scan_start`
    /// before the judging task is spawned, and removed by that task on ANY exit
    /// (finished, cancelled, or failed). The cancel route looks the run up here
    /// and calls `cancel()`; the judging loop checks the token before each LLM
    /// call and stops asking for more.
    ///
    /// ## Why a map in memory rather than a column on the row
    ///
    /// The thing being cancelled is a `tokio` task inside THIS process, so the
    /// handle that can stop it cannot outlive the process either. A `cancel_me`
    /// flag on `scan_runs` would have to be polled, would survive a restart the
    /// task did not, and would need its own cleanup. The durable half of the story
    /// is already on the row — the judging loop writes `status = 'cancelled'` on
    /// its way out — and this map is only the doorbell.
    ///
    /// An absent entry is therefore a real answer, not a gap: it means no live task
    /// in this process owns that run, and the cancel route refuses rather than
    /// reporting a stop that nothing will ever perform (Standing Rule 1).
    ///
    /// ## Rust Learning: `tokio::sync::Mutex`, not `std::sync::Mutex`
    ///
    /// Both would work — every critical section here is three lines of map access
    /// with no `.await` inside it — but `std::sync::Mutex::lock()` returns a
    /// `Result` because of POISONING: if any thread panics while holding the lock,
    /// every later `lock()` fails forever. The only ways to handle that are
    /// `.unwrap()` (banned in production paths) or an error arm for a case that
    /// cannot be recovered from. `tokio::sync::Mutex` has no poisoning, so
    /// `lock().await` yields the guard directly and there is no `Result` to
    /// mishandle. Every caller is already `async`.
    ///
    /// `Arc` because `AppState` is CLONED per request and every clone must reach
    /// the SAME map — a per-clone copy would let the route ring a doorbell in a
    /// house nobody is in.
    pub scan_cancel: Arc<Mutex<HashMap<Uuid, CancellationToken>>>,

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

    /// The question chat's engine (CC_TASK_CHAT_ENGINE_v1): ONE HTTP client for
    /// every chat turn, built at boot. `None` when there is no `ANTHROPIC_API_KEY`
    /// — every chat request then answers a named 503 instead of failing in flight.
    pub chat_engine: Option<Arc<dyn colossus_chat::ChatBackend>>,

    /// The chat package's measured size, remembered across requests
    /// (CC_TASK_CHAT_COST_FIX_v1 item 3). Filled lazily by the first chat turn,
    /// not at boot, so a provider outage delays one chat rather than the whole
    /// backend (ruled 2026-09-23, Q3).
    ///
    /// ## Rust Learning: `Arc` because `AppState` is CLONED per request
    ///
    /// A bare `PrefixSizeCache` would be copied into every clone, so a count
    /// taken on one request could never be seen by the next — the memo would
    /// never hit and every turn would pay a round trip. `Arc` makes the clone a
    /// refcount bump and gives every request a handle to the SAME cache. The
    /// interior `RwLock` is what then permits a write through the shared
    /// reference; see the module's own note.
    pub chat_prefix_size: Arc<crate::services::chat_prefix_size::PrefixSizeCache>,
    /// When the Admin "keep loaded" button last pinged successfully, since boot.
    /// In memory only and reset on restart (CC_TASK_KEEPWARM_BUTTON_v1); `Arc`
    /// for the reason `chat_prefix_size` gives above.
    pub keepwarm_last_ping: Arc<crate::services::chat_keepwarm_button::LastPing>,
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
// serde: allows unknown fields because this struct is built PROGRAMMATICALLY in
// `load_schema_metadata` from an already-parsed `ExtractionSchema` — it is never
// deserialized from a request body or a file. `deny_unknown_fields` would guard a
// boundary that does not exist here, and the `Deserialize` derive is present only
// so the type can round-trip through the schema API response.
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
// serde: allows unknown fields — see `SchemaMetadata`; this is one of its fields
// and shares its construction path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTypeInfo {
    pub name: String,
    pub description: String,
}

/// A relationship type defined in the extraction schema.
// serde: allows unknown fields — see `SchemaMetadata`; this is one of its fields
// and shares its construction path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipTypeInfo {
    pub name: String,
    pub description: String,
}
