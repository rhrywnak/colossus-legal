use axum::http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, ORIGIN};
use axum::http::HeaderName;
use axum::{routing::get, Json, Router};
use hyper::http::{HeaderValue, Method};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

use colossus_legal_backend::{
    api,
    config::AppConfig,
    neo4j::{check_neo4j, create_neo4j_graph},
    state::AppState,
};

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
    // Logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Load .env (NEO4J_URI, NEO4J_USER, NEO4J_PASSWORD, BACKEND_PORT, etc.)
    dotenvy::dotenv().ok();

    // Configuration and Neo4j connection
    let config = AppConfig::from_env().expect("Failed to load configuration");

    let graph = create_neo4j_graph(&config)
        .await
        .expect("Failed to connect to Neo4j");

    check_neo4j(&graph)
        .await
        .expect("Neo4j connectivity check failed");

    // PostgreSQL connection pool for analytical data (ratings, feedback).
    // PgPoolOptions configures the pool; .connect() opens it eagerly.
    // sqlx::migrate!() embeds .sql files at compile time, runs them on startup.
    let pg_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&config.postgres_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    sqlx::migrate!("./migrations")
        .run(&pg_pool)
        .await
        .expect("Failed to run PostgreSQL migrations");

    tracing::info!("PostgreSQL connected and migrations complete");

    // Build the RAG pipeline from config (if API key is available).
    //
    // ## Rust Learning: Graceful degradation with Option
    //
    // If the Anthropic API key is missing, we can't build the synthesizer,
    // so we set rag_pipeline = None. The /ask endpoint checks this and
    // returns 503 Service Unavailable. All other endpoints work fine.
    let rag_pipeline = build_rag_pipeline(&config, &graph).await;

    // Shared HTTP client with timeouts — reused across all handlers.
    // reqwest::Client pools connections internally, so sharing one client
    // is both faster and safer than creating a new one per request.
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .connect_timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("failed to build HTTP client");

    // Shared application state (global AppState)
    let state = AppState {
        graph,
        config,
        rag_pipeline,
        http_client,
        pg_pool,
    };

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
        .map(|s| {
            HeaderValue::from_str(&s)
                .unwrap_or_else(|_| panic!("Invalid CORS origin: {}", s))
        })
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
    // - /health
    // - /api/status
    // - everything from api::router()
    let app = Router::new()
        .route("/api/status", get(api_status))
        .merge(api::router())
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
) -> Option<Arc<colossus_rag::RagPipeline>> {
    use colossus_rag::{
        LegalAssembler, Neo4jExpander, QdrantRetriever, RagPipeline,
        RigSynthesizer, RuleBasedRouter,
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

    let retriever = QdrantRetriever::new(
        embedding_model,
        qdrant_client,
        "colossus_evidence",
        0.0, // No score threshold — let the assembler handle ranking
    );

    // --- Expander: Neo4j graph traversal ---
    let expander = Neo4jExpander::new(Arc::new(graph.clone()));

    // --- Assembler: legal context formatting ---
    // Custom system prompt with markdown FORMATTING rules appended.
    // The base prompt + RULES come from the default; we override to add
    // FORMATTING instructions so Claude returns markdown-structured answers.
    let assembler = LegalAssembler::with_system_prompt(SYSTEM_PROMPT);

    // --- Synthesizer: Claude via rig-core ---
    let synthesizer = match RigSynthesizer::claude(api_key, &config.anthropic_model, 4096) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to create RigSynthesizer: {e}");
            return None;
        }
    };

    // --- Wire everything together ---
    match RagPipeline::builder()
        .router(Box::new(router))
        .retriever(Box::new(retriever))
        .expander(Box::new(expander))
        .assembler(Box::new(assembler))
        .synthesizer(Box::new(synthesizer))
        .max_context_tokens(6000)
        .search_limit(10)
        .build()
    {
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
// System prompt with markdown formatting rules
// ---------------------------------------------------------------------------

const SYSTEM_PROMPT: &str = r#"You are a legal research assistant analyzing case evidence.

You have been given evidence from a case knowledge graph, including verbatim quotes from sworn testimony, court filings, and documentary evidence. Each piece of evidence includes its source document and page number where available.

RULES:
1. Answer using ONLY the provided evidence. Do not infer facts not present in the evidence.
2. For every factual claim in your answer, cite the specific evidence ID in parentheses, e.g., (evidence-phillips-q73).
3. When evidence items contradict each other, note the contradiction explicitly and identify which party made each statement.
4. If the provided evidence does not contain enough information to answer the question, say so clearly. Do not speculate.
5. Use plain language accessible to a non-lawyer, but maintain legal precision for citations.
6. When describing patterns (e.g., "Phillips repeatedly..."), list each specific instance with its citation.

FORMATTING:
- Use markdown formatting in your response.
- Use **bold** for key names, dates, and legal terms on first mention.
- Use ## headers to organize multi-part answers into clear sections.
- Use > blockquotes for verbatim quotes from evidence.
- Use numbered or bulleted lists when presenting multiple items.
- Keep paragraphs focused — one main point per paragraph.
- Do NOT use # (h1) headers — start with ## (h2) at the highest level.
- Do NOT over-format. If the answer is a single paragraph, just write the paragraph without headers or lists."#;