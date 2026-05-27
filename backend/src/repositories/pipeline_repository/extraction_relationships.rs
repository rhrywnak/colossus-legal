//! Extraction-relationship repository functions.
//!
//! Owns the `extraction_relationships` row type, the relationship CRUD,
//! and the per-pass result writers (`store_entities_and_relationships`
//! for pass 1, `store_pass2_relationships` for pass 2) that translate a
//! parsed LLM JSON response into rows on this table (plus
//! `extraction_items` via [`super::extraction_items::insert_extraction_item`]
//! for pass 1).
//!
//! The endpoint-shape tolerance helper `resolve_relationship_fields` is
//! private to this module — both stores share it so the schema-compliant
//! vs. short-form preference stays in one place.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::models::document_status::{REVIEW_STATUS_APPROVED, RUN_STATUS_COMPLETED};

use super::extraction_items::insert_extraction_item;
use super::PipelineRepoError;

// ── Record type ──────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExtractionRelationshipRecord {
    pub id: i32,
    pub run_id: i32,
    pub document_id: String,
    pub from_item_id: i32,
    pub to_item_id: i32,
    pub relationship_type: String,
    pub properties: Option<serde_json::Value>,
    pub review_status: String,
    pub tier: i32,
}

// ── CRUD ─────────────────────────────────────────────────────────

/// Insert an extraction relationship.
#[allow(clippy::too_many_arguments)]
pub async fn insert_extraction_relationship(
    pool: &PgPool,
    run_id: i32,
    document_id: &str,
    from_item_id: i32,
    to_item_id: i32,
    relationship_type: &str,
    properties: Option<&serde_json::Value>,
    tier: i32,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        r#"INSERT INTO extraction_relationships
           (run_id, document_id, from_item_id, to_item_id, relationship_type, properties, tier)
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
    )
    .bind(run_id)
    .bind(document_id)
    .bind(from_item_id)
    .bind(to_item_id)
    .bind(relationship_type)
    .bind(properties)
    .bind(tier)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get all extraction relationships for a document.
pub async fn get_all_relationships(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<ExtractionRelationshipRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ExtractionRelationshipRecord>(
        "SELECT * FROM extraction_relationships WHERE document_id = $1 ORDER BY id",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get approved extraction relationships for a document across every
/// COMPLETED extraction run (pass 1 + pass 2).
///
/// Ingest needs relationships from BOTH passes: pass-1 relationships
/// live under pass-1's `run_id`, pass-2 relationships under pass-2's.
/// Filtering by a single `run_id` loses half the graph once pass 2 is
/// enabled. Endpoints must still resolve to an approved item via the
/// inner joins — pass-2 relationships reference pass-1 items, and
/// those items carry the review_status the caller wants to respect.
///
/// The join against `extraction_runs` scopes to COMPLETED runs so a
/// partial / failed retry's orphan relationships never leak into Neo4j.
pub async fn get_approved_relationships_for_document_all_passes(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<ExtractionRelationshipRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ExtractionRelationshipRecord>(
        "SELECT r.* FROM extraction_relationships r
         JOIN extraction_runs rn ON rn.id = r.run_id
         JOIN extraction_items fi ON fi.id = r.from_item_id
         JOIN extraction_items ti ON ti.id = r.to_item_id
         WHERE r.document_id = $1
           AND rn.status = $2
           AND fi.review_status = $3
           AND ti.review_status = $3
         ORDER BY r.id",
    )
    .bind(document_id)
    .bind(RUN_STATUS_COMPLETED)
    .bind(REVIEW_STATUS_APPROVED)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get approved extraction relationships for a document's latest completed run.
///
/// Only returns relationships where both endpoints (from_item_id, to_item_id)
/// have review_status = 'approved'. This prevents ingesting relationships
/// that reference unapproved (potentially hallucinated) items.
pub async fn get_approved_relationships_for_document(
    pool: &PgPool,
    run_id: i32,
) -> Result<Vec<ExtractionRelationshipRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ExtractionRelationshipRecord>(
        "SELECT r.* FROM extraction_relationships r
         JOIN extraction_items fi ON fi.id = r.from_item_id
         JOIN extraction_items ti ON ti.id = r.to_item_id
         WHERE r.run_id = $1
           AND fi.review_status = 'approved'
           AND ti.review_status = 'approved'
         ORDER BY r.id",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get all extraction relationships for a specific run (by run_id).
pub async fn get_relationships_for_run(
    pool: &PgPool,
    run_id: i32,
) -> Result<Vec<ExtractionRelationshipRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ExtractionRelationshipRecord>(
        "SELECT * FROM extraction_relationships WHERE run_id = $1 ORDER BY id",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get relationships where BOTH endpoints are grounded (will be written to Neo4j).
pub async fn get_grounded_relationships_for_document(
    pool: &PgPool,
    run_id: i32,
) -> Result<Vec<ExtractionRelationshipRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ExtractionRelationshipRecord>(
        "SELECT r.* FROM extraction_relationships r
         JOIN extraction_items fi ON fi.id = r.from_item_id
         JOIN extraction_items ti ON ti.id = r.to_item_id
         WHERE r.run_id = $1
           AND fi.grounding_status IN ('exact', 'normalized', 'name_matched', 'heading_matched', 'derived', 'unverified')
           AND ti.grounding_status IN ('exact', 'normalized', 'name_matched', 'heading_matched', 'derived', 'unverified')
         ORDER BY r.id",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ── Endpoint-shape tolerance ─────────────────────────────────────

/// Resolve a relationship's endpoint + type fields, tolerating either
/// of the two conventions LLMs emit in practice.
///
/// The schema-compliant shape the templates specify is
/// `{from_entity, to_entity, relationship_type}` and that wins when
/// both conventions are present in the same object. The short form
/// `{from, to, type}` is Opus's natural JSON style and was causing
/// every relationship to be silently dropped on pass 2 before this
/// helper existed — every `get("from_entity")` returned `None`, the
/// fallback empty string failed `id_map` lookup, and the
/// skip-and-log branch fired for every row. Accepting both in one
/// place keeps the store functions aligned on one tolerance policy.
pub(crate) fn resolve_relationship_fields(rel: &serde_json::Value) -> (&str, &str, &str) {
    let from = rel
        .get("from_entity")
        .or_else(|| rel.get("from"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let to = rel
        .get("to_entity")
        .or_else(|| rel.get("to"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let rtype = rel
        .get("relationship_type")
        .or_else(|| rel.get("type"))
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN");
    (from, to, rtype)
}

// ── Result writers ───────────────────────────────────────────────

/// Store the entities and relationships contained in a raw LLM response into
/// `extraction_items` and `extraction_relationships` for a given run.
///
/// Inputs come from a parsed `serde_json::Value` shaped like:
///
/// ```json
/// {
///   "entities": [
///     { "id": "e1", "entity_type": "Party",
///       "properties": { "full_name": "Marie Awad" },
///       "verbatim_quote": "..." },
///     ...
///   ],
///   "relationships": [
///     { "from_entity": "e1", "to_entity": "e2",
///       "relationship_type": "MENTIONS",
///       "properties": { ... } },
///     ...
///   ]
/// }
/// ```
///
/// Returns `(entity_count, relationship_count)` — the number of rows inserted
/// into each table. Relationships whose `from_entity` or `to_entity` cannot
/// be resolved via the LLM-supplied `id` → DB `item_id` map are SKIPPED
/// (logged and ignored) rather than erroring, so partial outputs still
/// produce a usable graph. Unknown/missing JSON fields fall back to safe
/// defaults (`entity_type = "unknown"`, `relationship_type = "UNKNOWN"`).
///
/// ## Why this lives in the repository layer
///
/// Prior to 2026-04-16 this helper lived in `api::pipeline::chunk_storage`
/// (deleted by commit 1414838 as part of the P2-Cleanup purge of the old
/// chunked extraction path). The step-layer [`LlmExtract`] is the only
/// remaining caller, and a storage helper is a pure data-layer concern —
/// placing it here avoids re-introducing a step → api-handler dependency.
///
/// [`LlmExtract`]: crate::pipeline::steps::llm_extract::LlmExtract
pub async fn store_entities_and_relationships(
    pool: &sqlx::PgPool,
    run_id: i32,
    document_id: &str,
    parsed: &serde_json::Value,
) -> Result<(usize, usize), PipelineRepoError> {
    use std::collections::HashMap;

    let mut id_map: HashMap<String, i32> = HashMap::new();
    let mut entity_count: usize = 0;

    if let Some(entities) = parsed.get("entities").and_then(|v| v.as_array()) {
        for entity in entities {
            let entity_type = entity
                .get("entity_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            let json_id = entity.get("id").and_then(|v| v.as_str()).unwrap_or("");

            let verbatim_quote = entity
                .get("verbatim_quote")
                .and_then(|v| v.as_str())
                .or_else(|| {
                    entity
                        .get("properties")
                        .and_then(|p| p.get("verbatim_quote"))
                        .and_then(|v| v.as_str())
                });

            let item_id = insert_extraction_item(
                pool,
                run_id,
                document_id,
                entity_type,
                entity,
                verbatim_quote,
            )
            .await?;

            if !json_id.is_empty() {
                id_map.insert(json_id.to_string(), item_id);
            }
            entity_count += 1;
        }
    }

    let mut rel_count: usize = 0;

    if let Some(rels) = parsed.get("relationships").and_then(|v| v.as_array()) {
        for rel in rels {
            let (from_key, to_key, relationship_type) = resolve_relationship_fields(rel);

            let (Some(&from_id), Some(&to_id)) = (id_map.get(from_key), id_map.get(to_key)) else {
                tracing::warn!(
                    run_id, document_id,
                    from = %from_key, to = %to_key,
                    "Skipping relationship with unresolved endpoint(s)"
                );
                continue;
            };

            let properties = rel.get("properties");

            insert_extraction_relationship(
                pool,
                run_id,
                document_id,
                from_id,
                to_id,
                relationship_type,
                properties,
                1,
            )
            .await?;
            rel_count += 1;
        }
    }

    Ok((entity_count, rel_count))
}

/// Persist pass-2 relationships against the pass-2 `extraction_runs` row.
///
/// Pass 2's JSON is relationships-only (`{"relationships": [...]}`) and
/// its endpoints reference pass-1 entities by the LLM-authored id
/// (e.g. `"party-001"`). The caller builds `id_map` from the same
/// [`Pass1Entity`] list it injected into the prompt; this function
/// skips-and-logs any relationship whose endpoint cannot be resolved
/// (matching the partial-output tolerance of the pass-1 writer). The
/// `tier` is fixed at `1` to mirror pass 1 — downstream code already
/// treats tier 1 as "direct LLM extraction".
///
/// [`Pass1Entity`]: super::extraction_items_pass1::Pass1Entity
pub async fn store_pass2_relationships(
    pool: &PgPool,
    run_id: i32,
    document_id: &str,
    parsed: &serde_json::Value,
    id_map: &std::collections::HashMap<String, i32>,
) -> Result<usize, PipelineRepoError> {
    let mut rel_count: usize = 0;
    if let Some(rels) = parsed.get("relationships").and_then(|v| v.as_array()) {
        for rel in rels {
            let (from_key, to_key, relationship_type) = resolve_relationship_fields(rel);

            let (Some(&from_id), Some(&to_id)) = (id_map.get(from_key), id_map.get(to_key)) else {
                tracing::warn!(
                    run_id, document_id,
                    from = %from_key, to = %to_key,
                    "Pass 2: skipping relationship with unresolved endpoint(s)"
                );
                continue;
            };

            let properties = rel.get("properties");

            insert_extraction_relationship(
                pool,
                run_id,
                document_id,
                from_id,
                to_id,
                relationship_type,
                properties,
                1,
            )
            .await?;
            rel_count += 1;
        }
    }
    Ok(rel_count)
}

#[cfg(test)]
mod tests {
    //! Pure-function tests for the endpoint-shape tolerance helper.
    //! The DB-touching `store_*` paths are exercised by integration
    //! tests; the JSON-only `resolve_relationship_fields` projection
    //! can be asserted directly.
    use super::*;

    #[test]
    fn resolve_rel_fields_accepts_schema_compliant_form() {
        // The canonical shape the templates specify. Must keep working
        // — pass 1 on Sonnet produces this today.
        let rel = serde_json::json!({
            "from_entity": "allegation-007",
            "to_entity": "count-001",
            "relationship_type": "SUPPORTS",
        });
        let (from, to, rtype) = resolve_relationship_fields(&rel);
        assert_eq!(from, "allegation-007");
        assert_eq!(to, "count-001");
        assert_eq!(rtype, "SUPPORTS");
    }

    #[test]
    fn resolve_rel_fields_accepts_short_form() {
        // Opus's natural JSON style. Before the helper, every pass-2
        // relationship in this shape was silently dropped — the bug
        // this change fixes.
        let rel = serde_json::json!({
            "from": "admission-003",
            "to": "ctx:allegation-014",
            "type": "CORROBORATES",
        });
        let (from, to, rtype) = resolve_relationship_fields(&rel);
        assert_eq!(from, "admission-003");
        assert_eq!(to, "ctx:allegation-014");
        assert_eq!(rtype, "CORROBORATES");
    }

    #[test]
    fn resolve_rel_fields_long_form_wins_on_collision() {
        // An LLM could emit both forms in the same object (unlikely
        // but possible). The schema-compliant long form wins so the
        // behaviour matches the documented template shape.
        let rel = serde_json::json!({
            "from": "short-a",
            "from_entity": "long-a",
            "to": "short-b",
            "to_entity": "long-b",
            "type": "SHORT_TYPE",
            "relationship_type": "LONG_TYPE",
        });
        let (from, to, rtype) = resolve_relationship_fields(&rel);
        assert_eq!(from, "long-a");
        assert_eq!(to, "long-b");
        assert_eq!(rtype, "LONG_TYPE");
    }

    #[test]
    fn resolve_rel_fields_empty_when_neither_form_present() {
        // Defensive: a relationship with neither convention falls
        // through to empty endpoints, which the caller's id_map
        // lookup fails on and logs as "unresolved endpoints". The
        // `UNKNOWN` fallback for the type matches the pre-helper
        // behaviour so the audit trail stays stable.
        let rel = serde_json::json!({ "properties": { "note": "x" } });
        let (from, to, rtype) = resolve_relationship_fields(&rel);
        assert_eq!(from, "");
        assert_eq!(to, "");
        assert_eq!(rtype, "UNKNOWN");
    }
}
