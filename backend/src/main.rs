use axum::http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, ORIGIN};
use axum::http::HeaderName;
use axum::{routing::get, Json, Router};
use clap::{Parser, Subcommand};
use hyper::http::{HeaderValue, Method};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

use colossus_legal_backend::{
    api, cli,
    config::AppConfig,
    database,
    neo4j::{check_neo4j, create_neo4j_graph},
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

    // --- Load external prompt templates from disk ---
    //
    // Prompts are loaded once at startup. If the files don't exist,
    // the loader returns None and we fall back to compiled defaults.
    let prompts = prompt_loader::load_prompts(&config.prompts_dir);

    // Build the RAG pipeline from config (if API key is available).
    //
    // ## Rust Learning: Graceful degradation with Option
    //
    // If the Anthropic API key is missing, we can't build the synthesizer,
    // so we set rag_pipeline = None. The /ask endpoint checks this and
    // returns 503 Service Unavailable. All other endpoints work fine.
    let rag_pipeline = build_rag_pipeline(&config, &graph, &prompts).await;

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

    axum::serve(listener, app).await.expect("Server error");
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
) -> Option<Arc<colossus_rag::RagPipeline>> {
    use colossus_rag::{
        EmbeddingReranker, GraphDirectRetriever, LegalAssembler, Neo4jExpander, QdrantRetriever,
        RagPipeline, RigSynthesizer, RuleBasedRouter,
    };

    // Check for API key first — no key means no pipeline.
    let api_key = config.anthropic_api_key.as_deref()?;

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

    let fastembed_client = rig_fastembed::Client::new();
    let embedding_model = Arc::new(
        fastembed_client.embedding_model(&rig_fastembed::FastembedModel::NomicEmbedTextV15),
    );

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

    // Clone the embedding model Arc BEFORE passing to the retriever,
    // because QdrantRetriever::new() takes ownership of the Arc.
    // The reranker needs the same model (cheap Arc reference count bump).
    let reranker_model = embedding_model.clone();

    let retriever = QdrantRetriever::new(
        embedding_model,
        qdrant_client,
        "colossus_evidence",
        0.0, // No score threshold — let the assembler handle ranking
    );

    // --- Reranker: post-expansion semantic filtering ---
    //
    // Shares the same embedding model as the retriever (cheap Arc clone).
    // Filters graph-expanded chunks by cosine similarity to the question.
    let reranker = EmbeddingReranker::new(reranker_model, config.rerank_threshold);

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

    // --- Synthesizer: Claude via rig-core ---
    let synthesizer = match RigSynthesizer::claude(api_key, &config.anthropic_model, 4096) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to create RigSynthesizer: {e}");
            return None;
        }
    };

    // --- Graph Direct Retriever: for decomposed graph sub-queries ---
    let graph_retriever = GraphDirectRetriever::new(Arc::new(graph.clone()));

    // --- Wire everything together ---
    let mut builder = RagPipeline::builder()
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
