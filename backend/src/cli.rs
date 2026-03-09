//! CLI subcommand handlers.
//!
//! Extracted from main.rs to keep it under 300 lines.
//! Currently contains only the `embed` subcommand, which runs the
//! embedding pipeline directly (no HTTP server, no auth).

use crate::config::AppConfig;
use crate::services::{embedding_pipeline, qdrant_service};

/// Run the embedding pipeline as a CLI command.
///
/// This bypasses the HTTP server and auth layer entirely. It's designed
/// to be called via `podman exec` from Ansible/Semaphore for repeatable,
/// automated re-indexing.
///
/// Prints JSON result to stdout for machine parsing.
/// Exits with code 0 on success, 1 on failure.
pub async fn run_embed_command(
    config: &AppConfig,
    graph: &neo4rs::Graph,
    http_client: &reqwest::Client,
    clean: bool,
) {
    // If --clean flag, delete the collection first
    if clean {
        tracing::info!("--clean flag: deleting Qdrant collection before re-embedding");
        match qdrant_service::delete_collection(http_client, &config.qdrant_url).await {
            Ok(()) => tracing::info!("Qdrant collection deleted"),
            Err(e) => {
                // Collection might not exist — that's OK for a clean start
                tracing::warn!("Could not delete collection (may not exist): {e}");
            }
        }
    }

    tracing::info!("Starting embedding pipeline...");

    match embedding_pipeline::run_embedding_pipeline(
        graph,
        http_client,
        &config.qdrant_url,
        &config.fastembed_cache_path,
    )
    .await
    {
        Ok(result) => {
            // Print structured JSON to stdout for Ansible to parse
            let output = serde_json::json!({
                "status": "success",
                "total_nodes": result.total_nodes,
                "embedded_count": result.embedded_count,
                "nodes_by_type": result.nodes_by_type,
                "duration_seconds": result.duration_seconds,
                "errors": result.errors,
            });
            println!("{}", serde_json::to_string_pretty(&output).expect("JSON serialization failed"));
            std::process::exit(0);
        }
        Err(e) => {
            let output = serde_json::json!({
                "status": "error",
                "error": e.to_string(),
            });
            eprintln!("{}", serde_json::to_string_pretty(&output).expect("JSON serialization failed"));
            std::process::exit(1);
        }
    }
}
