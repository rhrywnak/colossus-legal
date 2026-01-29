#[derive(Debug, Clone)]
pub struct AppConfig {
    pub neo4j_uri: String,
    pub neo4j_user: String,
    pub neo4j_password: String,
    pub document_storage_path: String,
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

        Ok(Self {
            neo4j_uri,
            neo4j_user,
            neo4j_password,
            document_storage_path,
        })
    }
}
