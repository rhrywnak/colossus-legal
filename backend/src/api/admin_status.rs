//! GET /api/admin/status — Returns environment, version, and health summary.
//!
//! ## Rust Learning: Compile-Time Constants
//!
//! `env!("CARGO_PKG_VERSION")` reads the version from Cargo.toml at compile
//! time and embeds it as a `&'static str` in the binary. This means the version
//! string exists in the executable itself — no config file or env var needed
//! at runtime. The macro fails compilation if the env var doesn't exist,
//! which is fine because Cargo always sets CARGO_PKG_VERSION.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub environment: String,
    pub version: String,
    pub neo4j_connected: bool,
    pub qdrant_connected: bool,
    pub postgres_connected: bool,
}

/// GET /api/admin/status — Health check for all backend services.
///
/// Performs quick connectivity checks against Neo4j, Qdrant, and PostgreSQL.
/// Returns booleans so the admin UI can show green/red status dots.
pub async fn get_status(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<StatusResponse>, AppError> {
    require_admin(&user)?;

    // Neo4j — run a trivial Cypher query
    let neo4j_ok = state.graph.run(neo4rs::query("RETURN 1")).await.is_ok();

    // Qdrant — HTTP GET to the collections endpoint with a 3-second timeout
    let qdrant_ok = state
        .http_client
        .get(format!(
            "{}/collections/colossus_evidence",
            state.config.qdrant_url.trim_end_matches('/')
        ))
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    // PostgreSQL — trivial query
    let pg_ok = sqlx::query("SELECT 1")
        .execute(&state.pg_pool)
        .await
        .is_ok();

    Ok(Json(StatusResponse {
        environment: state.config.environment.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        neo4j_connected: neo4j_ok,
        qdrant_connected: qdrant_ok,
        postgres_connected: pg_ok,
    }))
}
