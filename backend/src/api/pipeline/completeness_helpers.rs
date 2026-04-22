//! Helpers for the entity-level completeness verification.
//!
//! ## Shift from counts to entity existence
//!
//! The previous design compared `extraction_items` counts against Neo4j
//! node counts and Qdrant point counts. That model is unsound for any
//! document whose entities are shared with another document: `MERGE` in
//! [`create_party_nodes`](super::ingest_helpers::create_party_nodes)
//! collapses spelling variants into one node, so pipeline counts
//! legitimately exceed Neo4j counts without indicating a bug.
//!
//! This module replaces that surface with entity-level verification:
//! for each approved item we compute the expected Neo4j id (the same id
//! Ingest would have written), batch-verify existence in Neo4j, and then
//! batch-verify a Qdrant point for each found node. The result carries
//! *which* ids are missing, not just counts.
//!
//! See `COMPLETENESS_VERIFICATION_REDESIGN_v1.md` §3 for the full data
//! flow and pass/fail semantics.

use std::collections::HashSet;

use neo4rs::{query, Graph};

use crate::error::AppError;
use crate::repositories::pipeline_repository::ExtractionItemRecord;
use crate::services::qdrant_service;

use super::ingest_helpers::{slug, stable_entity_id};

// ─────────────────────────────────────────────────────────────────────
// Expected-id computation
// ─────────────────────────────────────────────────────────────────────

/// Compute the expected Neo4j node id for each approved extraction item.
///
/// Party items (entity_type `"Person"` or `"Organization"` — the post-ingest
/// labels after `update_item_entity_type` runs) use the MERGE key
/// `create_party_nodes` writes: `person-{slug(name)}` or `org-{slug(name)}`.
/// The name comes from `item_data.properties.party_name` with a fallback
/// to `full_name`. Items missing both are skipped with a warning; the
/// completeness check surfaces them as "unverifiable" rather than a hard
/// failure — the data is malformed, not the ingest pipeline.
///
/// Non-Party items delegate to [`stable_entity_id`] — the same function
/// Ingest uses for `create_entity_node`. Bit-for-bit id match.
///
/// ## Blind spot (acknowledged in the design doc)
///
/// Cross-document name resolution in `create_party_nodes` can assign a
/// party a Neo4j id *other* than `person-{slug(name)}` — typically a
/// pre-existing node's id. This helper cannot reproduce resolution
/// without re-running it, so such items will surface as "missing". A
/// future improvement (storing `extraction_items.neo4j_id` at ingest
/// time) removes the blind spot; the current approach trades complete
/// coverage for a no-migration change.
///
/// Returns `(extraction_item.id, expected_neo4j_id)` pairs in input
/// order, minus any skipped items.
pub fn compute_expected_neo4j_ids(
    items: &[ExtractionItemRecord],
    doc_id: &str,
) -> Vec<(i32, String)> {
    let mut out: Vec<(i32, String)> = Vec::with_capacity(items.len());
    for item in items {
        // R1: prefer the persisted Neo4j id when Ingest recorded it.
        // This is the only branch that handles resolver-matched Parties
        // correctly — the recomputation paths below can't reproduce the
        // resolver's cross-document assignment.
        if let Some(ref neo4j_id) = item.neo4j_node_id {
            out.push((item.id, neo4j_id.clone()));
            continue;
        }
        // Fallback for legacy rows ingested before the R1 migration.
        match item.entity_type.as_str() {
            "Person" | "Organization" => {
                let props = &item.item_data["properties"];
                let name = props["party_name"]
                    .as_str()
                    .or_else(|| props["full_name"].as_str());
                let Some(name) = name else {
                    tracing::warn!(
                        item_id = item.id,
                        entity_type = %item.entity_type,
                        "completeness: skipping Party item with no party_name/full_name"
                    );
                    continue;
                };
                let prefix = if item.entity_type == "Organization" {
                    "org"
                } else {
                    "person"
                };
                out.push((item.id, format!("{prefix}-{}", slug(name))));
            }
            _ => {
                out.push((item.id, stable_entity_id(item, doc_id)));
            }
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────
// Neo4j verification
// ─────────────────────────────────────────────────────────────────────

/// Batch-verify a set of expected node ids against the Neo4j graph.
///
/// Single Cypher query with `UNWIND` + `OPTIONAL MATCH` so one round
/// trip covers every id. `OPTIONAL MATCH` returns a row for every input
/// regardless of match, so we can identify misses by iterating rows
/// where `found = false`.
///
/// Returns the ids that were NOT found. An empty input list returns an
/// empty result without making a query.
pub async fn verify_neo4j_nodes(
    graph: &Graph,
    expected_ids: &[String],
) -> Result<Vec<String>, AppError> {
    if expected_ids.is_empty() {
        return Ok(Vec::new());
    }
    let cypher = "UNWIND $ids AS expected_id \
                  OPTIONAL MATCH (n {id: expected_id}) \
                  RETURN expected_id, n IS NOT NULL AS found";
    let mut result = graph
        .execute(query(cypher).param("ids", expected_ids.to_vec()))
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j verify query failed: {e}"),
        })?;

    let mut missing: Vec<String> = Vec::new();
    while let Some(row) = result.next().await.map_err(|e| AppError::Internal {
        message: format!("Neo4j verify row fetch failed: {e}"),
    })? {
        let id: String = row.get("expected_id").unwrap_or_default();
        let found: bool = row.get("found").unwrap_or(false);
        if !found {
            missing.push(id);
        }
    }
    Ok(missing)
}

/// Verify the Document node exists in Neo4j for this document.
///
/// Ingest writes `d.source_document_id = doc_id` on the Document node.
/// Absence here is a FAIL — the document's entire graph is gone.
pub async fn document_node_exists(graph: &Graph, doc_id: &str) -> Result<bool, AppError> {
    let cypher = "MATCH (d:Document {source_document_id: $doc_id}) RETURN count(d) AS n";
    let mut result = graph
        .execute(query(cypher).param("doc_id", doc_id))
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j Document node query failed: {e}"),
        })?;
    let row = result
        .next()
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j Document node row fetch failed: {e}"),
        })?
        .ok_or_else(|| AppError::Internal {
            message: "Neo4j Document node query returned no row".to_string(),
        })?;
    let n: i64 = row.get("n").unwrap_or(0);
    Ok(n > 0)
}

// ─────────────────────────────────────────────────────────────────────
// Qdrant verification
// ─────────────────────────────────────────────────────────────────────

/// Batch-verify that a set of Neo4j node ids have Qdrant points.
///
/// Scrolls Qdrant with a single `node_id IN [...]` filter via
/// [`qdrant_service::scroll_node_ids_in`], then does a set difference
/// against the input list to identify node ids with no point.
///
/// Missing points are a WARN, not a FAIL — re-indexing repairs them.
pub async fn verify_qdrant_points(
    http_client: &reqwest::Client,
    qdrant_url: &str,
    node_ids: &[String],
) -> Result<Vec<String>, AppError> {
    if node_ids.is_empty() {
        return Ok(Vec::new());
    }
    let present: HashSet<String> =
        qdrant_service::scroll_node_ids_in(http_client, qdrant_url, node_ids)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Qdrant scroll failed: {e}"),
            })?;
    let mut missing: Vec<String> = Vec::new();
    for id in node_ids {
        if !present.contains(id) {
            missing.push(id.clone());
        }
    }
    Ok(missing)
}

// ─────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(id: i32, entity_type: &str, properties: serde_json::Value) -> ExtractionItemRecord {
        ExtractionItemRecord {
            id,
            run_id: 1,
            document_id: "doc-test".to_string(),
            entity_type: entity_type.to_string(),
            item_data: serde_json::json!({ "label": "test", "properties": properties }),
            verbatim_quote: None,
            grounding_status: None,
            grounded_page: None,
            review_status: "approved".to_string(),
            reviewed_by: None,
            reviewed_at: None,
            review_notes: None,
            graph_status: "written".to_string(),
            neo4j_node_id: None,
            resolved_entity_type: None,
        }
    }

    const DOC_ID: &str = "doc-awad-v-catholic-family-complaint-11-1-13";

    #[test]
    fn compute_ids_non_party_uses_stable_entity_id() {
        let item = make_item(
            1,
            "ComplaintAllegation",
            serde_json::json!({ "paragraph_number": "42" }),
        );
        let ids = compute_expected_neo4j_ids(std::slice::from_ref(&item), DOC_ID);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0].0, 1);
        // stable_entity_id for ComplaintAllegation returns {doc_slug}:para:{n}
        assert!(
            ids[0].1.ends_with(":para:42"),
            "expected `:para:42` suffix, got: {}",
            ids[0].1
        );
    }

    #[test]
    fn compute_ids_person_uses_name_slug() {
        let person = make_item(
            1,
            "Person",
            serde_json::json!({ "party_name": "Marie Awad" }),
        );
        let org = make_item(
            2,
            "Organization",
            serde_json::json!({ "party_name": "Catholic Family Services" }),
        );
        let ids = compute_expected_neo4j_ids(&[person, org], DOC_ID);
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0].1, "person-marie-awad");
        // Note: prefix is "org-" (not "organization-") because that's
        // what create_party_nodes actually writes.
        assert_eq!(ids[1].1, "org-catholic-family-services");
    }

    #[test]
    fn compute_ids_person_falls_back_to_full_name() {
        // Some schemas use `full_name` instead of `party_name`.
        let item = make_item(
            1,
            "Person",
            serde_json::json!({ "full_name": "Marie Awad" }),
        );
        let ids = compute_expected_neo4j_ids(std::slice::from_ref(&item), DOC_ID);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0].1, "person-marie-awad");
    }

    #[test]
    fn compute_ids_skips_person_with_no_name() {
        let item = make_item(1, "Person", serde_json::json!({ "role": "plaintiff" }));
        let ids = compute_expected_neo4j_ids(std::slice::from_ref(&item), DOC_ID);
        assert!(
            ids.is_empty(),
            "Party item missing both party_name and full_name must be skipped"
        );
    }

    #[test]
    fn compute_ids_uses_persisted_neo4j_node_id_when_present() {
        // R1: the persisted id short-circuits every recomputation branch.
        // Critical for resolver-matched Parties — the name would naively
        // slug to "person-mr-dalek", but the resolver assigned
        // "person-dalek" (from a different document). The persisted id
        // wins.
        let mut item = make_item(
            42,
            "Person",
            serde_json::json!({ "party_name": "Mr. Dalek" }),
        );
        item.neo4j_node_id = Some("person-dalek".to_string());
        let ids = compute_expected_neo4j_ids(std::slice::from_ref(&item), DOC_ID);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], (42, "person-dalek".to_string()));
    }
}
