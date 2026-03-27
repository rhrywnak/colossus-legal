use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub neo4j_uri: String,
    pub neo4j_user: String,
    pub neo4j_password: String,
    pub document_storage_path: String,
    pub qdrant_url: String,
    pub fastembed_cache_path: String,
    /// Anthropic API key — None means /ask returns 503 but the rest of the app works.
    pub anthropic_api_key: Option<String>,
    /// Claude model ID for synthesis (default: claude-sonnet-4-6).
    pub anthropic_model: String,
    /// Minimum cosine similarity for reranking graph-expanded nodes.
    /// Graph-expanded chunks below this threshold are filtered out.
    /// Default: 0.3 (conservative — keeps most chunks).
    pub rerank_threshold: f32,
    /// Model for query decomposition (fast model like Sonnet).
    /// Defaults to claude-sonnet-4-6 for speed and cost efficiency.
    pub decomposer_model: String,
    /// PostgreSQL connection URL for analytical data (ratings, feedback).
    pub postgres_url: String,
    /// Directory containing prompt template files (synthesis.md, decomposition.md).
    /// Default: `/data/documents/prompts`
    pub prompts_dir: PathBuf,
    /// PostgreSQL connection URL for the pipeline v2 database (clean room).
    /// Separate from postgres_url which connects to colossus_legal.
    pub pipeline_database_url: String,
    /// Path to extraction schema YAML files directory.
    pub extraction_schema_dir: String,
    /// Path to extraction prompt template files directory.
    pub extraction_template_dir: String,
    /// Deployment environment name (e.g. "dev", "prod").
    /// Read from COLOSSUS_ENVIRONMENT, defaults to "unknown".
    pub environment: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        let neo4j_uri =
            std::env::var("NEO4J_URI").map_err(|_| "Missing env var: NEO4J_URI".to_string())?;
        let neo4j_user =
            std::env::var("NEO4J_USER").map_err(|_| "Missing env var: NEO4J_USER".to_string())?;
        let neo4j_password = std::env::var("NEO4J_PASSWORD")
            .map_err(|_| "Missing env var: NEO4J_PASSWORD".to_string())?;

        let document_storage_path = std::env::var("DOCUMENT_STORAGE_PATH")
            .unwrap_or_else(|_| "./data/documents".to_string());

        // Qdrant vector database URL (used by H.1 embedding pipeline)
        let qdrant_url = std::env::var("QDRANT_URL")
            .unwrap_or_else(|_| "http://localhost:6333".to_string());

        // fastembed model cache directory (ONNX model weights stored here)
        let fastembed_cache_path = std::env::var("FASTEMBED_CACHE_PATH")
            .unwrap_or_else(|_| "/data/models".to_string());

        // Anthropic API key — optional so the app starts without it.
        // If absent, POST /ask returns 503 Service Unavailable.
        let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY").ok();

        let anthropic_model = std::env::var("ANTHROPIC_MODEL")
            .unwrap_or_else(|_| "claude-sonnet-4-6".to_string());

        let rerank_threshold: f32 = std::env::var("RERANK_THRESHOLD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.3);

        let decomposer_model = std::env::var("DECOMPOSER_MODEL")
            .unwrap_or_else(|_| "claude-sonnet-4-6".to_string());

        let postgres_url = std::env::var("DATABASE_URL")
            .map_err(|_| "Missing env var: DATABASE_URL".to_string())?;

        let prompts_dir = PathBuf::from(
            std::env::var("PROMPTS_DIR")
                .unwrap_or_else(|_| "/data/documents/prompts".to_string()),
        );

        let pipeline_database_url = std::env::var("PIPELINE_DATABASE_URL")
            .map_err(|_| "Missing env var: PIPELINE_DATABASE_URL".to_string())?;

        let extraction_schema_dir = std::env::var("EXTRACTION_SCHEMA_DIR")
            .unwrap_or_else(|_| "./extraction_schemas".to_string());

        let extraction_template_dir = std::env::var("EXTRACTION_TEMPLATE_DIR")
            .unwrap_or_else(|_| "./extraction_templates".to_string());

        let environment = std::env::var("COLOSSUS_ENVIRONMENT")
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(Self {
            neo4j_uri,
            neo4j_user,
            neo4j_password,
            document_storage_path,
            qdrant_url,
            fastembed_cache_path,
            anthropic_api_key,
            anthropic_model,
            rerank_threshold,
            decomposer_model,
            postgres_url,
            pipeline_database_url,
            extraction_schema_dir,
            extraction_template_dir,
            prompts_dir,
            environment,
        })
    }
}
