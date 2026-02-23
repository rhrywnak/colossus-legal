//! Qdrant vector database REST API client.
//!
//! We use reqwest to talk to Qdrant's HTTP API directly instead of the
//! qdrant-client crate. This keeps our dependency tree smaller and gives
//! us full control over the requests.
//!
//! ## Pattern: reqwest JSON calls
//! `client.put(url).json(&body).send().await?` does three things:
//! 1. Serializes `body` to JSON via serde
//! 2. Sets Content-Type: application/json
//! 3. Sends the HTTP request and returns a `Response`
//!
//! We then check `.status()` and optionally parse the response body.

use serde::Serialize;

/// The Qdrant collection where all evidence embeddings are stored.
const COLLECTION_NAME: &str = "colossus_evidence";

/// A point to upsert into Qdrant.
#[derive(Debug, Serialize)]
pub struct QdrantPoint {
    pub id: u64,
    pub vector: Vec<f32>,
    pub payload: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum QdrantError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Qdrant API error (status {status}): {body}")]
    Api { status: u16, body: String },
}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Ensure the `colossus_evidence` collection exists in Qdrant.
///
/// - If it already exists (HTTP 200), logs and skips.
/// - If not found (HTTP 404 or "not found" in body), creates it with
///   768-dim cosine vectors, then creates payload indexes on `node_id`
///   and `node_type`.
pub async fn ensure_collection(
    client: &reqwest::Client,
    qdrant_url: &str,
) -> Result<(), QdrantError> {
    let url = format!("{qdrant_url}/collections/{COLLECTION_NAME}");

    let resp = client.get(&url).send().await?;

    if resp.status().is_success() {
        tracing::info!("Qdrant collection '{}' already exists", COLLECTION_NAME);
        return Ok(());
    }

    // Collection doesn't exist — create it
    tracing::info!("Creating Qdrant collection '{}'", COLLECTION_NAME);

    let body = serde_json::json!({
        "vectors": {
            "size": 768,
            "distance": "Cosine"
        }
    });

    let resp = client.put(&url).json(&body).send().await?;
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(QdrantError::Api { status, body });
    }

    // Create payload indexes for efficient filtering
    create_payload_index(client, qdrant_url, "node_id", "keyword").await?;
    create_payload_index(client, qdrant_url, "node_type", "keyword").await?;

    tracing::info!("Qdrant collection '{}' created with indexes", COLLECTION_NAME);
    Ok(())
}

/// Upsert a batch of points into the collection.
///
/// Splits into sub-batches of 50 to avoid oversized payloads.
pub async fn upsert_points(
    client: &reqwest::Client,
    qdrant_url: &str,
    points: Vec<QdrantPoint>,
) -> Result<(), QdrantError> {
    let url = format!("{qdrant_url}/collections/{COLLECTION_NAME}/points");

    for chunk in points.chunks(50) {
        let body = serde_json::json!({ "points": chunk });

        let resp = client.put(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(QdrantError::Api { status, body });
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a payload index on the collection for efficient filtering.
async fn create_payload_index(
    client: &reqwest::Client,
    qdrant_url: &str,
    field_name: &str,
    field_schema: &str,
) -> Result<(), QdrantError> {
    let url = format!("{qdrant_url}/collections/{COLLECTION_NAME}/index");

    let body = serde_json::json!({
        "field_name": field_name,
        "field_schema": field_schema
    });

    let resp = client.put(&url).json(&body).send().await?;
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(QdrantError::Api { status, body });
    }

    Ok(())
}
