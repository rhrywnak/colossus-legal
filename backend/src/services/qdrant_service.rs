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

use serde::{Deserialize, Serialize};

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
// Search
// ---------------------------------------------------------------------------

/// A single search result from Qdrant, with payload fields extracted.
#[derive(Debug)]
pub struct SearchResult {
    pub node_id: String,
    pub node_type: String,
    pub title: String,
    pub score: f32,
    pub document_id: Option<String>,
    pub page_number: Option<String>,
}

/// Search for similar vectors in the collection.
///
/// Sends a POST to Qdrant's search endpoint with the query vector.
/// Optionally filters by `node_type` if `node_type_filter` is provided.
pub async fn search_points(
    client: &reqwest::Client,
    qdrant_url: &str,
    query_vector: Vec<f32>,
    limit: usize,
    node_type_filter: Option<Vec<String>>,
) -> Result<Vec<SearchResult>, QdrantError> {
    let url = format!("{qdrant_url}/collections/{COLLECTION_NAME}/points/search");

    let mut body = serde_json::json!({
        "vector": query_vector,
        "limit": limit,
        "with_payload": true,
    });

    // Add node_type filter if specified
    if let Some(types) = node_type_filter {
        if !types.is_empty() {
            body["filter"] = serde_json::json!({
                "must": [{
                    "key": "node_type",
                    "match": { "any": types }
                }]
            });
        }
    }

    let resp = client.post(&url).json(&body).send().await?;
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(QdrantError::Api { status, body: text });
    }

    // Parse Qdrant response: { "result": [ { "id", "score", "payload": {...} }, ... ] }
    let data: QdrantSearchResponse = resp.json().await?;

    let results = data
        .result
        .into_iter()
        .map(|hit| SearchResult {
            node_id: extract_string(&hit.payload, "node_id"),
            node_type: extract_string(&hit.payload, "node_type"),
            title: extract_string(&hit.payload, "title"),
            score: hit.score,
            document_id: extract_optional_string(&hit.payload, "document_id"),
            page_number: extract_optional_string(&hit.payload, "page_number"),
        })
        .collect();

    Ok(results)
}

/// Qdrant search response shape (only the fields we need).
#[derive(Deserialize)]
struct QdrantSearchResponse {
    result: Vec<QdrantSearchHit>,
}

#[derive(Deserialize)]
struct QdrantSearchHit {
    score: f32,
    payload: serde_json::Value,
}

/// Extract a string from a JSON payload, returning "" if missing.
fn extract_string(payload: &serde_json::Value, key: &str) -> String {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Extract an optional string from a JSON payload.
fn extract_optional_string(payload: &serde_json::Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
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
