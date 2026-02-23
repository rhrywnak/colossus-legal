#[derive(Debug, Clone)]
pub struct AppConfig {
    pub neo4j_uri: String,
    pub neo4j_user: String,
    pub neo4j_password: String,
    pub document_storage_path: String,
    pub qdrant_url: String,
    pub fastembed_cache_path: String,
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

        Ok(Self {
            neo4j_uri,
            neo4j_user,
            neo4j_password,
            document_storage_path,
            qdrant_url,
            fastembed_cache_path,
        })
    }
}
