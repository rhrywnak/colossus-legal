//! The vector half of a ranked gather: one filtered search of
//! `colossus_evidence`.
//!
//! ## ⚑ The prefix, and why getting it wrong is silent
//!
//! nomic-embed-text is an ASYMMETRIC model. Text is indexed under
//! `search_document:` and searched with `search_query:`, and the two are a
//! matched pair — both live in [`crate::services::embedding_text`], named, so
//! this module cannot drift from the fourteen places that write the other half.
//!
//! Using the wrong prefix does not fail. The embedding still returns 768
//! floats, Qdrant still returns 200 hits, the page still renders. The results
//! are just quietly worse, and there is no error, no empty list and no log line
//! to notice — the same failure family as the silent truncation L2a's addendum
//! caught. A test asserts the pairing.
//!
//! ## Why the party filter is an id list rather than a payload filter
//!
//! Qdrant's payload carries the node's PROPERTIES, copied from the graph node.
//! `ABOUT` is an EDGE, not a property, so there is no party field in the
//! payload to filter on — checked on the live collection, not assumed.
//!
//! So the filter is resolved once, in Postgres, against `evidence_search.about`
//! (L1's mirror of that same edge), and the resulting id set bounds BOTH reads.
//! That is better than it sounds: it means the vector read and the lexical read
//! see exactly the same universe, which is what makes their ranks comparable
//! and the conservation identity checkable. One filter, one source of truth.

use crate::services::embedding_text::QUERY_PREFIX;
use crate::services::qdrant_service::{QdrantError, COLLECTION_NAME};

/// Prefix a composed gather query for the query side of the model.
///
/// Pure, so the pairing is testable without the model or the network.
pub fn query_text(composed: &str) -> String {
    format!("{QUERY_PREFIX}{composed}")
}

/// The Qdrant request body for one filtered vector search.
///
/// Split out from the call so its shape can be asserted without a live Qdrant —
/// the filter is the part that silently returns the wrong pool when it is
/// wrong, and a wrong filter looks exactly like a thin corpus.
pub fn search_body(
    vector: &[f32],
    allowed_ids: Option<&[String]>,
    limit: usize,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "vector": vector,
        "limit": limit,
        "with_payload": true,
    });
    // `must` with a `match any` over node_id. An empty allowed set is passed
    // through as an empty `any`, which matches nothing — deliberately, because
    // "the filter admits no cards" is a real state and must not silently become
    // "no filter".
    if let Some(ids) = allowed_ids {
        body["filter"] = serde_json::json!({
            "must": [{ "key": "node_id", "match": { "any": ids } }]
        });
    }
    body
}

/// Search the evidence collection, returning evidence ids best-first.
///
/// Only the ids are returned: the fusion downstream works on rank alone, and
/// the card bodies are already in the mirror, so carrying payloads through
/// would be two copies of the same rows with a chance of disagreeing.
///
/// # Errors
/// Returns [`QdrantError`] if the request fails or Qdrant answers non-2xx.
pub async fn vector_search(
    client: &reqwest::Client,
    qdrant_url: &str,
    vector: &[f32],
    allowed_ids: Option<&[String]>,
    limit: usize,
) -> Result<Vec<String>, QdrantError> {
    let url = format!("{qdrant_url}/collections/{COLLECTION_NAME}/points/search");
    let resp = client
        .post(&url)
        .json(&search_body(vector, allowed_ids, limit))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(QdrantError::Api { status, body });
    }

    let data: serde_json::Value = resp.json().await?;
    let hits = data["result"].as_array().ok_or_else(|| QdrantError::Api {
        status: 200,
        // A 2xx whose body carries no `result` is a schema change, a proxy, or
        // a feature flag — never a legitimate empty search, which answers
        // `"result": []`. Collapsing it to an empty list would make a broken
        // Qdrant indistinguishable from a query that matched nothing.
        body: format!(
            "a 2xx search response carried no 'result' array: {}",
            excerpt_of(&data)
        ),
    })?;

    let ids = node_ids_of(hits);
    if ids.len() != hits.len() {
        // Rule 1: the skipped points are a real, distinct state. Counted here
        // rather than promised to a caller who has no way to see it — the
        // returned Vec is already filtered, so the loss is unrecoverable
        // downstream.
        tracing::warn!(
            returned = hits.len(),
            usable = ids.len(),
            dropped = hits.len() - ids.len(),
            "Qdrant returned points with no node_id payload; they cannot be joined to \
             any card and were dropped from the gather"
        );
    }
    Ok(ids)
}

// STRUCTURAL: how much of an unexpected response body is quoted into the error.
// A log/message format bound in the same family as
// `anthropic_stream::MALFORMED_PREVIEW_CHARS` — long enough to identify the
// shape that came back, short enough that a large body cannot flood the field.
const BODY_EXCERPT_CHARS: usize = 200;

/// The head of an unexpected response, for the error message.
fn excerpt_of(body: &serde_json::Value) -> String {
    body.to_string().chars().take(BODY_EXCERPT_CHARS).collect()
}

/// Pull `payload.node_id` out of a Qdrant search response, in rank order.
///
/// A hit with no `node_id` is SKIPPED rather than defaulted to an empty string:
/// a point with no node id cannot be joined to anything, and an empty id would
/// silently become a card nobody can open. The caller compares this list's
/// length against the hits it was given and warns on the difference — the
/// comparison is the mechanism, and it is at the call site because by the time
/// this returns the loss is no longer visible.
fn node_ids_of(hits: &[serde_json::Value]) -> Vec<String> {
    hits.iter()
        .filter_map(|hit| hit["payload"]["node_id"].as_str())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
#[path = "gather_vector_tests.rs"]
mod tests;
