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

use std::collections::HashSet;

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
    create_payload_index(client, qdrant_url, "document_id", "keyword").await?;
    create_payload_index(client, qdrant_url, "statement_type", "keyword").await?;
    create_payload_index(client, qdrant_url, "stated_by", "keyword").await?;
    create_payload_index(client, qdrant_url, "evidence_status", "keyword").await?;
    create_payload_index(client, qdrant_url, "category", "keyword").await?;

    tracing::info!("Qdrant collection '{}' ready with 7 payload indexes", COLLECTION_NAME);
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
///
/// Fields beyond the core three (node_id, node_type, title) are Optional
/// because they only exist on certain node types. For example, `stated_by`
/// only appears on Evidence nodes.
#[derive(Debug)]
pub struct SearchResult {
    pub node_id: String,
    pub node_type: String,
    pub title: String,
    pub score: f32,
    // Evidence-specific
    pub document_id: Option<String>,
    pub page_number: Option<String>,
    pub stated_by: Option<String>,
    pub statement_type: Option<String>,
    pub statement_date: Option<String>,
    pub exhibit_number: Option<String>,
    pub significance: Option<String>,
    pub verbatim_quote: Option<String>,
    // ComplaintAllegation-specific
    pub evidence_status: Option<String>,
    // Shared across types
    pub category: Option<String>,
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
            stated_by: extract_optional_string(&hit.payload, "stated_by"),
            statement_type: extract_optional_string(&hit.payload, "statement_type"),
            statement_date: extract_optional_string(&hit.payload, "statement_date"),
            exhibit_number: extract_optional_string(&hit.payload, "exhibit_number"),
            significance: extract_optional_string(&hit.payload, "significance"),
            verbatim_quote: extract_optional_string(&hit.payload, "verbatim_quote"),
            evidence_status: extract_optional_string(&hit.payload, "evidence_status"),
            category: extract_optional_string(&hit.payload, "category"),
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

/// Count points in the collection matching a payload filter.
///
/// Uses `POST /collections/{name}/points/count` with `exact: true`.
/// More efficient than scrolling all points when you only need the count.
pub async fn count_points_by_filter(
    client: &reqwest::Client,
    qdrant_url: &str,
    filter_key: &str,
    filter_value: &str,
) -> Result<usize, QdrantError> {
    let url = format!("{qdrant_url}/collections/{COLLECTION_NAME}/points/count");

    let body = serde_json::json!({
        "filter": {
            "must": [{
                "key": filter_key,
                "match": { "value": filter_value }
            }]
        },
        "exact": true
    });

    let resp = client.post(&url).json(&body).send().await?;
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(QdrantError::Api { status, body });
    }

    let data: serde_json::Value = resp.json().await?;
    let count = data["result"]["count"].as_u64().unwrap_or(0) as usize;
    Ok(count)
}

/// Delete all points matching a payload filter.
///
/// Uses `POST /collections/{name}/points/delete` with a filter body.
/// Returns the number of points that existed before deletion (via count).
pub async fn delete_points_by_filter(
    client: &reqwest::Client,
    qdrant_url: &str,
    filter_key: &str,
    filter_value: &str,
) -> Result<usize, QdrantError> {
    // Count first so we can report how many were removed
    let count = count_points_by_filter(client, qdrant_url, filter_key, filter_value).await?;
    if count == 0 {
        return Ok(0);
    }

    let url = format!("{qdrant_url}/collections/{COLLECTION_NAME}/points/delete");

    let body = serde_json::json!({
        "filter": {
            "must": [{
                "key": filter_key,
                "match": { "value": filter_value }
            }]
        }
    });

    let resp = client.post(&url).json(&body).send().await?;
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(QdrantError::Api { status, body });
    }

    Ok(count)
}

/// Delete the Qdrant collection. Used by the CLI `embed --clean` command.
///
/// Returns Ok(()) if deletion succeeded or collection didn't exist.
pub async fn delete_collection(
    client: &reqwest::Client,
    qdrant_url: &str,
) -> Result<(), QdrantError> {
    let url = format!("{qdrant_url}/collections/{COLLECTION_NAME}");

    let resp = client.delete(&url).send().await?;

    // 200 = deleted, 404 = didn't exist (both are fine for our purpose)
    if resp.status().is_success() || resp.status().as_u16() == 404 {
        Ok(())
    } else {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        Err(QdrantError::Api { status, body })
    }
}

/// Fetch all existing `node_id` values from the Qdrant collection.
///
/// Uses the scroll API with cursor-based pagination to handle large
/// collections efficiently. Requests only the `node_id` payload field
/// (no vectors) to minimize data transfer.
///
/// ## Rust Learning: Cursor-based pagination
///
/// Qdrant's scroll API returns a page of results plus an optional
/// `next_page_offset`. We loop until `next_page_offset` is `null`,
/// collecting results each iteration. This is the same concept as
/// SQL's OFFSET/LIMIT but using an opaque cursor — each response
/// tells you where to start the next request.
pub async fn get_existing_point_ids(
    client: &reqwest::Client,
    qdrant_url: &str,
) -> Result<HashSet<String>, QdrantError> {
    let url = format!("{qdrant_url}/collections/{COLLECTION_NAME}/points/scroll");
    let mut existing_ids = HashSet::new();
    let mut offset: Option<serde_json::Value> = None;

    loop {
        // Build the scroll request body.
        // `with_payload` accepts an "include" list — we only need `node_id`.
        // `with_vector` = false avoids fetching the 768-dim vectors.
        let mut body = serde_json::json!({
            "limit": 500,
            "with_payload": { "include": ["node_id"] },
            "with_vector": false,
        });

        // On subsequent pages, include the cursor from the previous response.
        if let Some(ref off) = offset {
            body["offset"] = off.clone();
        }

        let resp = client.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(QdrantError::Api { status, body: text });
        }

        let data: ScrollResponse = resp.json().await?;

        // Extract node_id from each point's payload.
        for point in &data.result.points {
            if let Some(node_id) = point
                .payload
                .get("node_id")
                .and_then(|v| v.as_str())
            {
                existing_ids.insert(node_id.to_string());
            }
        }

        // If next_page_offset is null, we've reached the end.
        match data.result.next_page_offset {
            Some(ref npo) if !npo.is_null() => {
                offset = Some(npo.clone());
            }
            _ => break,
        }
    }

    tracing::info!(
        "Scrolled {} existing point IDs from Qdrant",
        existing_ids.len()
    );
    Ok(existing_ids)
}

/// Response shape for Qdrant's scroll API.
#[derive(Deserialize)]
struct ScrollResponse {
    result: ScrollResult,
}

#[derive(Deserialize)]
struct ScrollResult {
    points: Vec<ScrollPoint>,
    next_page_offset: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct ScrollPoint {
    payload: serde_json::Value,
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
