use axum::{routing::get, Json, Router};
use hyper::http::{HeaderValue, Method};
use serde::Serialize;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
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

#[allow(dead_code)]
async fn health_check() -> &'static str {
    "OK"
}

async fn api_status() -> Json<StatusResponse> {
    Json(StatusResponse {
        app: "colossus-legal-backend",
        version: "0.1.0",
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

    // Shared application state (global AppState)
    // If your AppState has more fields, add them here.
    let state = AppState { graph };

    // Port
    let port: u16 = std::env::var("BACKEND_PORT")
        .unwrap_or_else(|_| "3403".to_string())
        .parse()
        .expect("Invalid BACKEND_PORT");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // CORS (dev-friendly; you can tighten this later)
    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:5473"),
            HeaderValue::from_static("http://localhost:3403"),
        ])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any);

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

    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind port");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}

