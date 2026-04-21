use axum::http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, ORIGIN};
use axum::http::HeaderName;
use axum::{routing::get, Json, Router};
use clap::{Parser, Subcommand};
use hyper::http::{HeaderValue, Method};
use serde::Serialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::watch;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

use colossus_extract::providers::AnthropicProvider;
use colossus_extract::LlmProvider;

use colossus_pipeline::worker::config::WorkerConfig;
use colossus_pipeline::Worker;

/// Model id the Chat endpoint uses when the request does not specify one.
///
/// Hardcoded rather than read from an env var so the default is explicit in
/// source and doesn't silently drift with `LLM_MODEL` (which drives pipeline
/// extraction). Pipeline extraction and Chat can legitimately differ.
const DEFAULT_CHAT_MODEL: &str = "claude-sonnet-4-6";

/// Per-chat-model `max_tokens` passed to `AnthropicProvider::new`. The
/// Chat endpoint always wraps the provider in `RigSynthesizer::new(_, 4096)`
/// at request time, so this default is only used if some future caller
/// invokes the provider directly.
const CHAT_MAX_TOKENS: u32 = 4096;

use colossus_legal_backend::{
    api, cli,
    config::AppConfig,
    database,
    neo4j::{check_neo4j, create_neo4j_graph},
    pipeline::context::{AppContext, AppContextDeps},
    pipeline::task::DocProcessing,
    prompt_loader,
    state::{AppState, EntityTypeInfo, RelationshipTypeInfo, SchemaMetadata},
};

/// Colossus-Legal backend server and admin tools.
///
/// ## Pattern: clap derive macro
/// The `#[derive(Parser)]` macro generates a CLI argument parser from the
/// struct definition. Each field becomes a CLI flag or subcommand.
/// `#[command(name = "colossus-backend")]` sets the binary name in help text.
/// When no subcommand is given, it defaults to `Serve` via `unwrap_or`.
#[derive(Parser)]
#[command(name = "colossus-backend", about = "Colossus-Legal backend")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the HTTP server (default)
    Serve,
    /// Run the embedding pipeline (Neo4j → fastembed → Qdrant)
    Embed {
        /// Delete the Qdrant collection before re-embedding (full re-index)
        #[arg(long)]
        clean: bool,

        /// Only embed nodes not already in Qdrant (default behavior).
        /// Ignored when --clean is passed.
        #[arg(long, default_value_t = true)]
        incremental: bool,

        /// Show what would be indexed without actually embedding
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Serialize)]
struct StatusResponse {
    app: &'static str,
    version: &'static str,
    status: &'static str,
}

async fn api_status() -> Json<StatusResponse> {
    Json(StatusResponse {
        app: "colossus-legal-backend",
        version: env!("CARGO_PKG_VERSION"),
        status: "ok",
    })
}

#[tokio::main]
async fn main() {
    // Parse CLI args
    let cli = Cli::parse();

    // Logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    // Load .env (NEO4J_URI, NEO4J_USER, NEO4J_PASSWORD, BACKEND_PORT, etc.)
    dotenvy::dotenv().ok();

    // Shared setup: config, Neo4j, HTTP client
    let config = AppConfig::from_env().expect("Failed to load configuration");

    let graph = create_neo4j_graph(&config)
        .await
        .expect("Failed to connect to Neo4j");

    check_neo4j(&graph)
        .await
        .expect("Neo4j connectivity check failed");

    // Run Neo4j schema migrations (uniqueness constraints for entity nodes).
    // These are idempotent (IF NOT EXISTS) and must run before any ingest.
    colossus_legal_backend::api::pipeline::graph_migrations::run_graph_migrations(&graph).await;

    // Shared HTTP client with timeouts — reused across all handlers.
    // reqwest::Client pools connections internally, so sharing one client
    // is both faster and safer than creating a new one per request.
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .connect_timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("failed to build HTTP client");

    // Dispatch on CLI command
    match cli.command.unwrap_or(Commands::Serve) {
        Commands::Serve => {
            run_serve(config, graph, http_client).await;
        }
        Commands::Embed {
            clean,
            incremental,
            dry_run,
        } => {
            // --clean overrides --incremental (full re-index)
            let incremental = incremental && !clean;
            cli::run_embed_command(&config, &graph, &http_client, clean, incremental, dry_run)
                .await;
        }
    }
}

/// Start the HTTP server (default command).
async fn run_serve(config: AppConfig, graph: neo4rs::Graph, http_client: reqwest::Client) {
    // Connect to both PostgreSQL databases and run migrations.
    // See database.rs for details on the two-pool / two-migration strategy.
    let db = database::init_pools(&config).await;
    let pg_pool = db.main_pool;
    let pipeline_pool = db.pipeline_pool;

    // --- Build AppContext for pipeline step execution ---
    //
    // AppContext holds the LLM provider, embedding provider, semaphore,
    // and all paths/handles the pipeline steps need. Construction reads
    // LLM_PROVIDER and EMBEDDING_PROVIDER env vars.
    //
    // Failure here is fatal — no pipeline without providers.
    let app_context = AppContext::from_deps_and_env(AppContextDeps {
        pipeline_pool: pipeline_pool.clone(),
        graph: graph.clone(),
        qdrant_url: config.qdrant_url.clone(),
        http_client: http_client.clone(),
        schema_dir: config.extraction_schema_dir.clone(),
        template_dir: config.extraction_template_dir.clone(),
        document_storage_path: config.document_storage_path.clone(),
        profile_dir: config.processing_profile_dir.clone(),
        system_prompt_dir: config.system_prompt_dir.clone(),
    })
    .expect("Failed to build AppContext from env");

    let app_context = Arc::new(app_context);
    tracing::info!(
        llm_model = app_context.llm_provider.model_name(),
        embed_model = app_context.embedding_provider.model_name(),
        llm_concurrency_limit = app_context.llm_semaphore.available_permits(),
        "AppContext constructed"
    );

    // --- Start the pipeline worker ---
    //
    // The worker polls pipeline_jobs and executes DocProcessing step
    // sequences. Worker::run() takes a watch::Receiver<bool>; when the
    // sender sends true, the worker completes its current job and exits.
    //
    // We spawn the worker on the tokio runtime as a background task and
    // keep the JoinHandle to await it on shutdown. The watch::Sender is
    // kept in scope so we can signal shutdown after axum's graceful
    // shutdown future completes.
    let worker_config = WorkerConfig::from_env();
    tracing::info!(
        worker_id = %worker_config.worker_id,
        max_concurrent = worker_config.max_concurrent,
        "Worker config loaded"
    );

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let worker = Worker::<DocProcessing>::new(
        pipeline_pool.clone(),
        app_context.clone(),
        worker_config,
        shutdown_rx,
    );
    let worker_handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
        match worker.run().await {
            Ok(()) => tracing::info!("Worker exited cleanly"),
            Err(e) => tracing::error!(error = %e, "Worker exited with error"),
        }
    });

    // --- Load external prompt templates from disk ---
    //
    // Prompts are loaded once at startup. If the files don't exist,
    // the loader returns None and we fall back to compiled defaults.
    let prompts = prompt_loader::load_prompts(&config.prompts_dir);

    // Build the per-model Chat provider map. Each entry is an
    // `AnthropicProvider` with `temperature = None` (natural variation —
    // distinct from pipeline extraction which pins to Some(0.0) for
    // determinism). Empty when ANTHROPIC_API_KEY is unset.
    let chat_providers = build_chat_providers(&config, &pipeline_pool).await;
    if !chat_providers.is_empty() && !chat_providers.contains_key(DEFAULT_CHAT_MODEL) {
        tracing::error!(
            default = DEFAULT_CHAT_MODEL,
            available = ?chat_providers.keys().collect::<Vec<_>>(),
            "Chat default model not present in provider map — /ask with \
             no `model` field will return 400 until the llm_models row is \
             added or activated"
        );
    }

    // Build the RAG pipeline from config (if API key is available).
    //
    // ## Rust Learning: Graceful degradation with Option
    //
    // If the Anthropic API key is missing, we can't build the synthesizer,
    // so we set rag_pipeline = None. The /ask endpoint checks this and
    // returns 503 Service Unavailable. All other endpoints work fine.
    //
    // The pipeline's built-in synthesizer is taken from the Chat default
    // provider so `ask()` and `ask_with_synthesizer(…default…)` agree on
    // temperature semantics. Falls back to `llm_provider_from_env()` if
    // the Chat default isn't in the provider map (typically because the
    // `llm_models` row is missing) so admin paths still work.
    let default_chat_provider = chat_providers.get(DEFAULT_CHAT_MODEL).cloned();
    let rag_pipeline =
        build_rag_pipeline(&config, &graph, &prompts, default_chat_provider).await;

    // Audit log repository — records every admin action for accountability.
    let audit_repo = colossus_legal_backend::repositories::audit_repository::AuditRepository::new(
        pg_pool.clone(),
    );

    // --- Load extraction schema metadata from YAML ---
    //
    // The schema defines what entity types and relationship types exist.
    // This metadata is served to the frontend via GET /api/schema and used
    // by the query layer to understand the graph structure.
    let schema_metadata = load_schema_metadata(&config);

    // Construct the embedding provider from environment variables.
    // See colossus_extract::providers for EMBEDDING_PROVIDER semantics.
    // Panicking via expect() here is correct behavior: if the embedding
    // provider can't be built, the server can't serve embeddings — fail
    // fast at startup rather than fail per-request later.
    let embedding_provider = colossus_extract::providers::embedding_provider_from_env()
        .expect("Failed to construct embedding provider — check EMBEDDING_PROVIDER env var");

    // Shared application state (global AppState)
    let state = AppState {
        graph,
        config,
        rag_pipeline,
        http_client,
        pg_pool,
        pipeline_pool,
        audit_repo,
        embedding_provider,
        schema_metadata,
        chat_providers,
        default_chat_model: DEFAULT_CHAT_MODEL.to_string(),
    };

    // Ensure the Qdrant collection exists with the correct dimensions.
    // Running this at startup (before any handler can run) makes the
    // collection's dimensionality deterministic: the value baked in is
    // whatever the provider reports right now, not whatever the first
    // incoming request happened to supply.
    //
    // If the collection already exists (common case on DEV/PROD where a
    // previous deployment created it), ensure_collection short-circuits
    // on the HTTP 200 path and logs "already exists".
    if let Err(e) = colossus_legal_backend::services::qdrant_service::ensure_collection(
        &state.http_client,
        &state.config.qdrant_url,
        state.embedding_provider.dimensions(),
    )
    .await
    {
        tracing::error!(error = %e, "Failed to ensure Qdrant collection at startup — continuing anyway; handlers may retry");
    }

    // Port
    let port: u16 = std::env::var("BACKEND_PORT")
        .unwrap_or_else(|_| "3403".to_string())
        .parse()
        .expect("Invalid BACKEND_PORT");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // CORS — configurable via CORS_ALLOWED_ORIGINS env var (comma-separated).
    // Falls back to localhost defaults for local development.
    //
    // RUST NOTE — from_static vs from_str:
    // from_static() requires a &'static str (compile-time string literal).
    // from_str() accepts a &str (runtime string). Since we read from an env
    // var, the values are runtime strings, so we use from_str().
    let cors_origins: Vec<HeaderValue> = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| {
            "http://localhost:5473,http://localhost:3403,http://10.10.0.99:5473".to_string()
        })
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(|s| HeaderValue::from_str(&s).unwrap_or_else(|_| panic!("Invalid CORS origin: {}", s)))
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(cors_origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            AUTHORIZATION,
            ACCEPT,
            CONTENT_TYPE,
            ORIGIN,
            HeaderName::from_static("x-authentik-username"),
            HeaderName::from_static("x-authentik-email"),
            HeaderName::from_static("x-authentik-groups"),
            HeaderName::from_static("x-authentik-name"),
        ])
        .allow_credentials(true);

    // Build router:
    // - /health at root (standard convention, no /api/ prefix)
    // - /api/status, plus everything from api::router() under /api/
    //
    // ## Rust Learning: nest() vs merge()
    // .merge() combines two routers at the same level (routes keep their paths).
    // .nest("/api", router) prepends "/api" to every route in the sub-router.
    // This ensures ALL API routes get the /api/ prefix structurally —
    // you can't accidentally add a route without it.
    let app = Router::new()
        .route("/health", get(api::health_check))
        .route("/api/status", get(api_status))
        .nest("/api", api::router())
        .layer(cors)
        .with_state(state);
    tracing::info!("Starting colossus-legal backend on http://{}", addr);

    let listener = TcpListener::bind(addr).await.expect("Failed to bind port");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");

    tracing::info!("HTTP server stopped; signalling worker shutdown");

    // Signal the worker to stop.
    let _ = shutdown_tx.send(true);

    // Wait up to 30 seconds for the worker to drain.
    // This ceiling matches podman's default stop-timeout (10s) plus headroom.
    // Past this, we exit regardless; the recovery system will clean up
    // any jobs that were in-flight on the next restart.
    match tokio::time::timeout(std::time::Duration::from_secs(30), worker_handle).await {
        Ok(Ok(())) => tracing::info!("Worker drain completed"),
        Ok(Err(e)) => tracing::error!(error = %e, "Worker task panicked during drain"),
        Err(_) => tracing::warn!("Worker drain timed out after 30s; exiting anyway"),
    }

    tracing::info!("colossus-legal backend shutdown complete");
}

/// Await a shutdown signal — either Ctrl+C or SIGTERM (unix).
///
/// This is the axum-canonical shutdown pattern:
/// <https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs>
///
/// Podman sends SIGTERM on `podman stop`, so SIGTERM handling is REQUIRED
/// for graceful shutdown under our deployment. Without it, podman SIGKILLs
/// the process 10 seconds after SIGTERM, leaving the worker's in-flight
/// jobs zombie'd for the recovery system to clean up on next restart.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("shutdown: Ctrl+C received"),
        _ = terminate => tracing::info!("shutdown: SIGTERM received"),
    }
}

// ---------------------------------------------------------------------------
// RAG pipeline construction
// ---------------------------------------------------------------------------

/// Build the colossus-rag pipeline from environment config.
///
/// Returns `None` if the Anthropic API key is not configured (graceful
/// degradation — the rest of the app still works, only /ask returns 503).
///
/// ## Rust Learning: Why `async`?
///
/// This function is async even though most component construction is sync,
/// because we need to be inside the tokio runtime to set up the Qdrant gRPC
/// connection (qdrant-client uses tokio internally).
///
/// ## Pipeline components wired here:
///
/// 1. **RuleBasedRouter** — keyword-based routing with Awad v CFS aliases
/// 2. **QdrantRetriever** — embeds via rig-fastembed, searches via Qdrant gRPC
/// 3. **Neo4jExpander** — traverses Neo4j relationships from seed nodes
/// 4. **LegalAssembler** — formats chunks into a context prompt
/// 5. **RigSynthesizer** — calls Claude API via rig-core
async fn build_rag_pipeline(
    config: &AppConfig,
    graph: &neo4rs::Graph,
    prompts: &prompt_loader::LoadedPrompts,
    default_chat_provider: Option<Arc<dyn LlmProvider>>,
) -> Option<Arc<colossus_rag::RagPipeline>> {
    use colossus_rag::{
        EmbeddingReranker, GraphDirectRetriever, LegalAssembler, Neo4jExpander, QdrantRetriever,
        RagPipeline, RigSynthesizer, RuleBasedRouter,
    };

    // Preserve the legacy short-circuit: if no Anthropic API key is
    // configured, /ask has no synthesis backend and we build no pipeline.
    if config.anthropic_api_key.is_none() {
        return None;
    }

    // LLM provider for the pipeline's built-in synthesizer. Prefer the
    // Chat default provider (temperature = None, natural variation) so
    // `ask()` matches `ask_with_synthesizer(…default…)`. Fall back to
    // `llm_provider_from_env()` only if the Chat default isn't in the map
    // (typically because the `llm_models` row is missing) — that path
    // picks up `LLM_TEMPERATURE` from the environment and may therefore
    // diverge from the chat-default's None.
    let llm_provider = match default_chat_provider {
        Some(p) => p,
        None => match colossus_extract::providers::llm_provider_from_env() {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Failed to build LLM provider for RAG: {e}");
                return None;
            }
        },
    };
    let embedding_provider =
        match colossus_extract::providers::embedding_provider_from_env() {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Failed to build embedding provider for RAG: {e}");
                return None;
            }
        };

    // --- Router: rule-based with legal case aliases ---
    let router = RuleBasedRouter::legal_defaults();

    // --- Retriever: fastembed + Qdrant gRPC ---
    //
    // ## Rust Learning: gRPC vs REST for Qdrant
    //
    // The old Minerva pipeline used Qdrant's REST API (port 6333) via reqwest.
    // colossus-rag uses the official qdrant-client crate which speaks gRPC
    // (port 6334) — faster for batch operations and type-safe. We derive
    // the gRPC URL from the existing REST URL in config.
    let qdrant_grpc_url = config.qdrant_url.replace(":6333", ":6334");

    let qdrant_client = match qdrant_client::Qdrant::from_url(&qdrant_grpc_url)
        .skip_compatibility_check()
        .build()
    {
        Ok(client) => Arc::new(client),
        Err(e) => {
            tracing::error!("Failed to create Qdrant gRPC client: {e}");
            return None;
        }
    };

    // Both the retriever and the reranker take an `Arc<dyn EmbeddingProvider>`
    // in v0.10.4. Cloning an Arc is cheap (one refcount bump) so we share a
    // single provider instance across both stages.
    let retriever = QdrantRetriever::new(
        embedding_provider.clone(),
        qdrant_client,
        "colossus_evidence",
        0.0, // No score threshold — let the assembler handle ranking
    );

    // --- Reranker: post-expansion semantic filtering ---
    let reranker = EmbeddingReranker::new(embedding_provider, config.rerank_threshold);

    // --- Expander: Neo4j graph traversal ---
    let expander = Neo4jExpander::new(Arc::new(graph.clone()));

    // --- Assembler: legal context formatting ---
    // Use file-loaded prompt if available, otherwise the compiled default.
    // The default includes 7 RULES + FORMATTING section for markdown output.
    let synthesis_prompt = prompts
        .synthesis
        .as_deref()
        .unwrap_or(prompt_loader::DEFAULT_SYNTHESIS_PROMPT);
    let assembler = LegalAssembler::with_system_prompt(synthesis_prompt);

    // --- Synthesizer: uses the shared LLM provider (v0.10.4 API) ---
    //
    // The model id and provider backend are resolved by
    // `llm_provider_from_env()` (via LLM_PROVIDER / ANTHROPIC_MODEL env
    // vars), so `config.anthropic_model` is no longer consulted here.
    let synthesizer = RigSynthesizer::new(llm_provider, 4096);

    // --- Graph Direct Retriever: for decomposed graph sub-queries ---
    let graph_retriever = GraphDirectRetriever::new(Arc::new(graph.clone()));

    // --- Wire everything together ---
    let builder = RagPipeline::builder()
        .router(Box::new(router))
        .retriever(Box::new(retriever))
        .expander(Box::new(expander))
        .assembler(Box::new(assembler))
        .synthesizer(Box::new(synthesizer))
        .reranker(reranker)
        .graph_retriever(graph_retriever)
        .max_context_tokens(6000)
        .search_limit(10);

    // TODO(Phase2): LlmDecomposer reconstructed from rag_config DB table
    // TODO(Phase2): build_rag_pipeline() rewritten to use Arc<dyn LlmProvider>

    match builder.build() {
        Ok(pipeline) => {
            tracing::info!("RAG pipeline initialized successfully");
            Some(Arc::new(pipeline))
        }
        Err(e) => {
            tracing::error!("Failed to build RAG pipeline: {e}");
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Chat provider map
// ---------------------------------------------------------------------------

/// Build one `AnthropicProvider` per active `llm_models` row with
/// `provider = "anthropic"`, keyed by the model id.
///
/// Temperature is `None` on every entry — chat responses should have
/// natural variation. Pipeline extraction uses `Some(0.0)` via its own
/// `pipeline::providers::provider_for_model` helper.
///
/// Returns an empty map if `ANTHROPIC_API_KEY` is unset or if the DB
/// query fails. Both are non-fatal: the `/ask` handler will surface a
/// missing default model as 400, and `/chat/models` still serves the
/// catalog from the DB directly.
async fn build_chat_providers(
    config: &AppConfig,
    pipeline_pool: &sqlx::PgPool,
) -> HashMap<String, Arc<dyn LlmProvider>> {
    use colossus_legal_backend::repositories::pipeline_repository::models;

    let mut map: HashMap<String, Arc<dyn LlmProvider>> = HashMap::new();

    let api_key = match &config.anthropic_api_key {
        Some(k) => k.clone(),
        None => {
            tracing::info!(
                "ANTHROPIC_API_KEY not set; chat provider map will be empty"
            );
            return map;
        }
    };

    let models = match models::list_active_models(pipeline_pool).await {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(error = %e, "Failed to load active llm_models for chat provider map");
            return map;
        }
    };

    for model in &models {
        if model.provider != "anthropic" {
            // Non-Anthropic chat backends (vLLM, future others) are out of
            // scope for this map; chat dispatch only supports Anthropic
            // today. Extraction continues to route vLLM through its own
            // provider_for_model.
            continue;
        }
        match AnthropicProvider::new(
            api_key.clone(),
            model.id.clone(),
            CHAT_MAX_TOKENS,
            None, // natural variation for chat
        ) {
            Ok(provider) => {
                map.insert(model.id.clone(), Arc::new(provider) as Arc<dyn LlmProvider>);
            }
            Err(e) => {
                tracing::error!(
                    model = %model.id, error = %e,
                    "Failed to construct AnthropicProvider for chat — skipping"
                );
            }
        }
    }

    tracing::info!(
        count = map.len(),
        models = ?map.keys().collect::<Vec<_>>(),
        "Chat provider map built"
    );
    map
}

// ---------------------------------------------------------------------------
// Schema metadata loading
// ---------------------------------------------------------------------------

/// Load extraction schema YAML and extract entity/relationship type metadata.
///
/// ## Rust Learning: expect() at startup
///
/// We use `expect()` here because this runs once during startup. If the schema
/// file is missing or malformed, the server should fail fast with a clear error
/// rather than starting in a broken state.
fn load_schema_metadata(config: &AppConfig) -> SchemaMetadata {
    use colossus_extract::ExtractionSchema;
    use std::path::Path;

    let schema_path = Path::new(&config.extraction_schema_dir).join("general_legal.yaml");

    let extraction_schema = ExtractionSchema::from_file(&schema_path).unwrap_or_else(|e| {
        panic!(
            "Failed to load extraction schema from {}: {:?}",
            schema_path.display(),
            e
        )
    });

    let entity_types: Vec<EntityTypeInfo> = extraction_schema
        .entity_types
        .iter()
        .map(|et| EntityTypeInfo {
            name: et.name.clone(),
            description: et.description.clone(),
        })
        .collect();

    let relationship_types: Vec<RelationshipTypeInfo> = extraction_schema
        .relationship_types
        .iter()
        .map(|rt| RelationshipTypeInfo {
            name: rt.name.clone(),
            description: rt.description.clone(),
        })
        .collect();

    tracing::info!(
        "Loaded extraction schema '{}': {} entity types, {} relationship types",
        extraction_schema.document_type,
        entity_types.len(),
        relationship_types.len(),
    );

    SchemaMetadata {
        document_type: extraction_schema.document_type,
        entity_types,
        relationship_types,
    }
}
