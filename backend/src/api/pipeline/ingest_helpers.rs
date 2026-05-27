//! Helper functions for the graph ingest endpoint.
//! Extracted from `ingest.rs` to keep it under 300 lines.
//!
//! ## Rust Learning: Generic entity node creation
//!
//! The `create_entity_node` function creates Neo4j nodes for ANY entity type.
//! The entity_type string from the extraction schema becomes the Neo4j label
//! directly (e.g., "ComplaintAllegation" → `:ComplaintAllegation`).
//!
//! Party entities are special: they use MERGE (upsert) and split into
//! `:Person` or `:Organization` based on the `entity_kind`/`party_type` property.

use std::collections::{HashMap, HashSet};

use neo4rs::query;
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::models::document_status::{
    ENTITY_COMPLAINT_ALLEGATION, ENTITY_HARM, ENTITY_LEGAL_COUNT, ENTITY_ORGANIZATION,
    ENTITY_PERSON, PARTY_SUBTYPES, REL_CONTAINED_IN, STATUS_INGESTED,
};
use crate::repositories::pipeline_repository::ExtractionItemRecord;

use super::ingest_resolver::ResolutionMap;

/// Generate a stable, URL-friendly slug from a name.
/// Lowercases for natural dedup: `"MARIE AWAD"` and `"Marie Awad"` both → `"marie-awad"`.
pub fn slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Generate a stable, content-derived ID for a Neo4j entity node.
///
/// ## Why stable IDs are required for MERGE idempotency
///
/// MERGE only avoids duplicates when the MERGE key (the `id` property) is
/// the same across runs. LlamaIndex documents this explicitly: "As long as
/// the ID of the node is the same, we can avoid duplicating data."
///
/// A counter-based ID like "complaint-allegation-001" changes if extraction
/// order changes (which LLMs do). MERGE on an unstable ID creates a new
/// node instead of updating the existing one.
///
/// ## ID scheme
///
/// IDs are derived from stable structural properties of the entity:
/// - ComplaintAllegation: {doc_slug}:para:{paragraph_number}
///   paragraph_number is a structural property of legal complaints —
///   numbered paragraphs are stable, they don't change between extractions.
/// - LegalCount: count-{count_number}  (case-global — NO doc_slug prefix)
///   Legal counts are numbered (Count I, II, III) and are case-global: the
///   same id is shared across every document and matches the canonical
///   loader's `count-{N}` (authored.rs `legal_count_entity_id`), so an
///   extracted LegalCount MERGEs onto the canonical node instead of
///   duplicating it.
/// - Harm: {doc_slug}:harm:{sha256(harm_type + description)[0..8]}
///   Harms are derived entities without natural numbers. A content hash
///   provides a stable fingerprint. 8 hex chars = 32-bit space, sufficient
///   for the small number of harms per document (typically 3-10).
/// - All other types: {doc_slug}:{entity_type_slug}:{sha256(item_data)[0..8]}
///   Fallback for unknown entity types introduced by future schemas.
///
/// The {doc_slug} prefix scopes document-specific entities to their document,
/// preventing ID collisions across different documents that happen to have
/// the same paragraph number. LegalCount is the deliberate exception — it is
/// case-global (no prefix) so extracted counts resolve onto the shared
/// canonical node.
pub fn stable_entity_id(item: &ExtractionItemRecord, doc_id: &str) -> String {
    let doc_slug = slug(doc_id);

    match item.entity_type.as_str() {
        ENTITY_COMPLAINT_ALLEGATION => {
            // Paragraph reference priority:
            //   1. `paragraph_number` (v2/v3 schemas) — string or integer
            //   2. `paragraph_ref`    (v4 schemas)    — string or integer
            //   3. Hash of allegation body text — first `summary` (v2/v3),
            //      then `allegation_text` (v4). Without this fallback, v4
            //      documents whose paragraph fields are missing would all
            //      collapse to the empty-string hash `hash-e3b0c442`,
            //      MERGEing every allegation into a single Neo4j node.
            let props = &item.item_data["properties"];
            let para = props["paragraph_number"]
                .as_str()
                .map(|s| s.to_string())
                .or_else(|| props["paragraph_number"].as_u64().map(|n| n.to_string()))
                .or_else(|| props["paragraph_ref"].as_str().map(|s| s.to_string()))
                .or_else(|| props["paragraph_ref"].as_u64().map(|n| n.to_string()))
                .unwrap_or_else(|| {
                    let body = props["summary"]
                        .as_str()
                        .or_else(|| props["allegation_text"].as_str())
                        .unwrap_or("");
                    let hash = format!("{:x}", Sha256::digest(body.as_bytes()));
                    format!("hash-{}", &hash[..8])
                });
            format!("{}:para:{}", doc_slug, para)
        }
        ENTITY_LEGAL_COUNT => {
            // Domain note: a LegalCount is case-global, not document-scoped.
            // Count I ("Breach of Fiduciary Duty") is the same legal count
            // whether the complaint, a motion, or a brief references it. The
            // canonical loader authors these nodes with id `count-{N}` (see
            // authored.rs `legal_count_entity_id`); producing the same id here
            // means an extracted LegalCount MERGEs onto the existing canonical
            // node at ingest rather than creating a duplicate. This is why —
            // unlike the other arms — we deliberately drop the `doc_slug`
            // prefix: two documents that both cite Count 1 must resolve to one
            // node.
            let count = item.item_data["properties"]["count_number"]
                .as_u64()
                .map(|n| n.to_string())
                .or_else(|| {
                    item.item_data["properties"]["count_number"]
                        .as_str()
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| {
                    // No usable count_number — fall back to a content hash of
                    // legal_basis so malformed extractions don't all collapse
                    // onto one id. Still un-prefixed (case-global); the
                    // `hash-` segment keeps it from ever colliding with a real
                    // `count-{N}` canonical node.
                    let legal_basis = item.item_data["properties"]["legal_basis"]
                        .as_str()
                        .unwrap_or("");
                    let hash = format!("{:x}", Sha256::digest(legal_basis.as_bytes()));
                    format!("hash-{}", &hash[..8])
                });
            // CONST: the `count-` prefix is a fixed cross-tier schema
            // identifier — the same literal the Tier-1 loader emits
            // (authored.rs `legal_count_entity_id`) and the Tier-2 setter
            // stamps (cypher.rs `set_legal_count_id`). It is part of the MERGE
            // contract, not an env-configurable value (Standing Rule 2).
            format!("count-{count}")
        }
        ENTITY_HARM => {
            let harm_type = item.item_data["properties"]["harm_type"]
                .as_str()
                .unwrap_or("");
            let description = item.item_data["properties"]["description"]
                .as_str()
                .unwrap_or("");
            let hash_input = format!("{}{}{}", doc_id, harm_type, description);
            let hash = format!("{:x}", Sha256::digest(hash_input.as_bytes()));
            format!("{}:harm:{}", doc_slug, &hash[..8])
        }
        other => {
            // Unknown entity type — hash the full item_data for uniqueness.
            let data_str = serde_json::to_string(&item.item_data).unwrap_or_default();
            let hash = format!("{:x}", Sha256::digest(data_str.as_bytes()));
            format!("{}:{}:{}", doc_slug, slug(other), &hash[..8])
        }
    }
}

/// Create or update the Document node in Neo4j. Returns the generated neo4j ID.
///
/// Uses MERGE on a stable ID derived from doc_id (not title) to ensure
/// idempotency. Re-processing the same document updates the existing
/// Document node instead of creating a duplicate.
pub async fn create_document_node(
    txn: &mut neo4rs::Txn,
    doc_id: &str,
    title: &str,
    doc_type: &str,
) -> Result<String, AppError> {
    // Document ID: use doc_id directly (already stable — it's the pipeline ID).
    // doc_id already starts with "doc-" (e.g. "doc-jeffrey-humphrey-affidavit"),
    // so slug() preserves that. Previously format!("doc-{}", slug(doc_id))
    // produced a double-prefixed id like "doc-doc-jeffrey-humphrey-affidavit" (B6).
    let neo4j_id = slug(doc_id);

    txn.run(
        query(
            "MERGE (d:Document {id: $id}) \
             ON CREATE SET d.title = $title, \
                           d.source_document_id = $source_id, \
                           d.doc_type = $doc_type, \
                           d.status = $status, \
                           d.ingested_at = datetime() \
             ON MATCH SET  d.title = $title, \
                           d.doc_type = $doc_type, \
                           d.status = $status, \
                           d.updated_at = datetime()",
        )
        .param("id", neo4j_id.as_str())
        .param("title", title)
        .param("source_id", doc_id)
        .param("doc_type", doc_type)
        .param("status", STATUS_INGESTED),
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to merge Document node: {e}"),
    })?;
    Ok(neo4j_id)
}

/// Create or merge Party nodes (Person/Organization) using entity resolution.
///
/// ## Rust Learning: MERGE for cross-document entity resolution
///
/// Parties are the only entity type that uses MERGE instead of CREATE.
/// The same person (e.g., "Marie Awad") can appear in multiple documents.
/// MERGE matches on the node ID and either creates a new node or updates
/// the existing one by appending the new document to `source_documents`.
pub async fn create_party_nodes(
    txn: &mut neo4rs::Txn,
    items: &[ExtractionItemRecord],
    doc_id: &str,
    pg_to_neo4j: &mut HashMap<i32, String>,
    pg_to_label: &mut HashMap<i32, String>,
    resolution_map: &ResolutionMap,
) -> Result<(usize, usize), AppError> {
    let mut seen: HashSet<String> = HashSet::new();
    let (mut persons, mut orgs) = (0usize, 0usize);

    // R4: accept items whose effective entity_type is Party or its
    // resolved forms. Fresh items carry "Party" (from the LLM); rows
    // seen after Ingest's `update_item_entity_type` call carry the
    // COALESCE'd resolved label. Extending the filter keeps the step
    // idempotent across those states.
    for item in items
        .iter()
        .filter(|i| PARTY_SUBTYPES.contains(&i.entity_type.as_str()))
    {
        let props = &item.item_data["properties"];
        // Support both property naming conventions across schemas
        let name = props["party_name"]
            .as_str()
            .or_else(|| props["full_name"].as_str())
            .unwrap_or("unknown");
        let role = props["role"].as_str().unwrap_or("");
        // Support both "party_type" (complaint.yaml) and "entity_kind" (general_legal.yaml)
        let party_type = props["party_type"]
            .as_str()
            .or_else(|| props["entity_kind"].as_str())
            .unwrap_or("individual");

        let is_org = party_type == "organization" || party_type.to_lowercase().contains("org");
        let label = if is_org {
            ENTITY_ORGANIZATION
        } else {
            ENTITY_PERSON
        };

        // Look up resolved ID from the resolution map
        let neo4j_id = resolution_map
            .get(name)
            .map(|(id, _)| id.clone())
            .unwrap_or_else(|| {
                let prefix = if is_org { "org" } else { "person" };
                format!("{prefix}-{}", slug(name))
            });

        pg_to_neo4j.insert(item.id, neo4j_id.clone());
        pg_to_label.insert(item.id, label.to_string());

        // Skip if we already MERGE'd this node in this batch
        if !seen.insert(neo4j_id.clone()) {
            continue;
        }

        let cypher = format!(
            "MERGE (n:{label} {{id: $id}}) \
             ON CREATE SET n.name = $name, n.role = $role, \
               n.source_document = $doc, n.source_documents = [$doc] \
             ON MATCH SET n.source_documents = CASE \
               WHEN NOT $doc IN coalesce(n.source_documents, []) \
               THEN coalesce(n.source_documents, []) + $doc \
               ELSE n.source_documents END"
        );
        txn.run(
            query(&cypher)
                .param("id", neo4j_id.as_str())
                .param("name", name)
                .param("role", role)
                .param("doc", doc_id),
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to merge {label} '{name}': {e}"),
        })?;

        if is_org {
            orgs += 1;
        } else {
            persons += 1;
        }
    }
    Ok((persons, orgs))
}

/// Create or update a Neo4j node for any non-Party entity type.
///
/// Uses MERGE on a stable content-derived ID to ensure idempotency.
/// Running this function twice with the same entity produces one node,
/// not two. This is the correct behavior for a re-processable pipeline.
///
/// ## Why MERGE instead of CREATE
///
/// Verified in production implementations:
/// - neo4j-graphrag-python: Neo4jWriter._upsert_nodes uses MERGE
/// - LlamaIndex: PropertyGraphStore.upsert_nodes uses MERGE
/// - neo4all: "entities are written to Neo4j via idempotent MERGE"
///
/// ## Why stable_entity_id instead of a sequence counter
///
/// LlamaIndex documents: "As long as the ID of the node is the same,
/// we can avoid duplicating data." A counter-based ID changes if
/// extraction order changes; a content-derived ID is stable.
pub async fn create_entity_node(
    txn: &mut neo4rs::Txn,
    item: &ExtractionItemRecord,
    doc_id: &str,
    _seq: usize, // kept for API compatibility but no longer used for ID generation
) -> Result<String, AppError> {
    let entity_type = &item.entity_type;

    // Validate label: alphanumeric + underscore only (prevent Cypher injection)
    if !entity_type.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(AppError::BadRequest {
            message: format!("Invalid entity type for Neo4j label: '{entity_type}'"),
            details: serde_json::json!({ "entity_type": entity_type }),
        });
    }

    // Generate stable content-derived ID
    let neo4j_id = stable_entity_id(item, doc_id);

    // Extract standard fields
    let title = item.item_data["label"].as_str().unwrap_or("").to_string();
    let verbatim_quote = item.verbatim_quote.as_deref().unwrap_or("").to_string();
    let grounding_status = item.grounding_status.as_deref().unwrap_or("").to_string();
    let page_number = item.grounded_page;

    // MERGE the node with core identity properties.
    // ON CREATE sets all fields for new nodes.
    // ON MATCH updates mutable fields for existing nodes.
    let cypher = format!(
        "MERGE (n:{entity_type} {{id: $id}}) \
         ON CREATE SET n.title = $title, \
                       n.source_document = $doc_id, \
                       n.verbatim_quote = $verbatim_quote, \
                       n.grounding_status = $grounding_status, \
                       n.created_at = datetime() \
         ON MATCH SET  n.title = $title, \
                       n.verbatim_quote = $verbatim_quote, \
                       n.grounding_status = $grounding_status, \
                       n.updated_at = datetime()"
    );

    txn.run(
        query(&cypher)
            .param("id", neo4j_id.as_str())
            .param("title", title.as_str())
            .param("doc_id", doc_id)
            .param("verbatim_quote", verbatim_quote.as_str())
            .param("grounding_status", grounding_status.as_str()),
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to merge {entity_type} '{neo4j_id}': {e}"),
    })?;

    // Set page_number if available
    if let Some(page) = page_number {
        txn.run(
            query(&format!(
                "MATCH (n:{entity_type} {{id: $id}}) SET n.page_number = $page"
            ))
            .param("id", neo4j_id.as_str())
            .param("page", page),
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to set page_number on {neo4j_id}: {e}"),
        })?;
    }

    // Set schema-defined properties from item_data["properties"]
    if let Some(props) = item.item_data.get("properties").and_then(|p| p.as_object()) {
        for (key, value) in props {
            // Skip properties already set above
            if key == "verbatim_quote" || key == "page_number" {
                continue;
            }
            // Validate property name (prevent Cypher injection)
            if !key.chars().all(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }

            let set_cypher = format!("MATCH (n:{entity_type} {{id: $id}}) SET n.{key} = $val");

            let set_q = match value {
                serde_json::Value::String(s) => Some(
                    query(&set_cypher)
                        .param("id", neo4j_id.as_str())
                        .param("val", s.as_str()),
                ),
                serde_json::Value::Number(n) => n
                    .as_i64()
                    .map(|i| {
                        query(&set_cypher)
                            .param("id", neo4j_id.as_str())
                            .param("val", i)
                    })
                    .or_else(|| {
                        n.as_f64().map(|f| {
                            query(&set_cypher)
                                .param("id", neo4j_id.as_str())
                                .param("val", f)
                        })
                    }),
                serde_json::Value::Bool(b) => Some(
                    query(&set_cypher)
                        .param("id", neo4j_id.as_str())
                        .param("val", *b),
                ),
                _ => None,
            };

            if let Some(set_q) = set_q {
                txn.run(set_q).await.map_err(|e| AppError::Internal {
                    message: format!("Failed to set property '{key}' on {neo4j_id}: {e}"),
                })?;
            }
        }
    }

    Ok(neo4j_id)
}

/// Validate that a relationship write carries the three required v5.1
/// provenance properties.
///
/// ## Why this exists
///
/// Per LEGAL_DATA_MODEL_v5_1.md §5.4 every relationship in Neo4j carries
/// three provenance properties: `source_document_id`, `extraction_run_id`,
/// and `created_at`. This validator is a defensive guard against future
/// regressions: if a refactor drops a property at a call site we'd
/// silently write a malformed edge and not notice until weeks later
/// during a Cypher query. Failing loudly here turns the bug into a
/// transaction abort with a clear error message that names the missing
/// property and the relationship type it was on.
///
/// ## Rust Learning: `Result<(), AppError>`
///
/// Returning `Result<(), AppError>` (a "unit Result") is the Rust idiom
/// for "this operation may fail but produces no value on success". The
/// `()` is the unit type — a zero-byte placeholder. Callers use `?` to
/// propagate the error or `?;` to propagate-and-discard the unit value.
///
/// `created_at` is not validated here — it is set unconditionally by
/// Cypher's `datetime()` and therefore cannot be empty.
fn validate_relationship_provenance(
    rel_type: &str,
    source_document_id: &str,
    extraction_run_id: &str,
) -> Result<(), AppError> {
    if source_document_id.is_empty() {
        return Err(AppError::Internal {
            message: format!(
                "Relationship write missing required property 'source_document_id' \
                 on relationship type '{rel_type}'. All relationships must carry \
                 source_document_id, extraction_run_id, and created_at per v5.1 §5.4."
            ),
        });
    }
    if extraction_run_id.is_empty() {
        return Err(AppError::Internal {
            message: format!(
                "Relationship write missing required property 'extraction_run_id' \
                 on relationship type '{rel_type}'. All relationships must carry \
                 source_document_id, extraction_run_id, and created_at per v5.1 §5.4."
            ),
        });
    }
    Ok(())
}

/// Build the Cypher string for a generic relationship write that stamps
/// the three v5.1 provenance properties.
///
/// ## Rust Learning: extracting a pure helper for testability
///
/// `txn.run` is async and hits Neo4j — that makes any test of the
/// calling async function require integration infrastructure. By
/// extracting the Cypher string assembly into a pure synchronous
/// function, unit tests can assert the SET clauses are present using
/// `String::contains` without spinning up Neo4j. This mirrors the
/// `mergeOverridesIntoResolved` pattern from the frontend (commit
/// f6bc936): extract the testable part, leave the I/O-bound part to
/// integration verification.
///
/// `rel_type` is interpolated directly into the Cypher template. The
/// caller is responsible for validating that `rel_type` contains only
/// alphanumeric characters and underscores (Cypher injection guard) —
/// see `create_ingest_relationship`.
///
/// ## ON MATCH coalesce semantics — first-wins
///
/// On a re-MERGE the relationship already exists. We use
/// `coalesce(existing, new)` rather than blindly overwriting because:
/// - `created_at` should be when the edge was FIRST created, not the
///   latest ingest timestamp — coalesce preserves the original.
/// - `source_document_id` and `extraction_run_id` describe the run that
///   first created the edge. "Most recent run that touched this edge"
///   is an audit-log question, not a graph-property question.
/// - Free backfill: legacy edges (pre-v5.1) have NULL on these
///   properties. The first re-MERGE after this lands populates them via
///   coalesce — so the migration tail handles itself incrementally.
fn build_relationship_with_provenance_cypher(rel_type: &str) -> String {
    format!(
        "MATCH (a {{id: $from_id}}), (b {{id: $to_id}}) \
         MERGE (a)-[r:{rel_type}]->(b) \
         ON CREATE SET r.source_document_id = $source_document_id, \
                       r.extraction_run_id = $extraction_run_id, \
                       r.created_at = datetime() \
         ON MATCH SET  r.source_document_id = coalesce(r.source_document_id, $source_document_id), \
                       r.extraction_run_id = coalesce(r.extraction_run_id, $extraction_run_id), \
                       r.created_at = coalesce(r.created_at, datetime()) \
         RETURN b.id"
    )
}

/// Create or update a relationship between two nodes inside a transaction.
///
/// Uses MERGE instead of CREATE to ensure idempotency — re-processing
/// the same document does not create duplicate relationships.
/// Zero rows from MATCH = broken ID mapping = hard error (rolls back).
///
/// Stamps the three v5.1 provenance properties on every write
/// (`source_document_id`, `extraction_run_id`, `created_at`). See
/// `build_relationship_with_provenance_cypher` for the coalesce-on-match
/// rationale.
///
/// `extraction_run_id` should be pre-formatted by the caller (typically
/// `format!("run-{}", rel.run_id)` for pipeline-extracted relationships).
/// Pre-formatting at the call site keeps the prefix convention in one
/// place per ingest path and frees this helper from the i32-vs-string
/// concern.
pub async fn create_ingest_relationship(
    txn: &mut neo4rs::Txn,
    from_id: &str,
    to_id: &str,
    rel_type: &str,
    source_document_id: &str,
    extraction_run_id: &str,
) -> Result<(), AppError> {
    // Validate rel_type to prevent Cypher injection
    if !rel_type.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(AppError::BadRequest {
            message: format!("Invalid relationship type: '{rel_type}'"),
            details: serde_json::json!({ "rel_type": rel_type }),
        });
    }

    // v5.1 provenance contract — fail loudly if either of the two string
    // properties is empty. `created_at` is set by Cypher datetime() and
    // is not validated here.
    validate_relationship_provenance(rel_type, source_document_id, extraction_run_id)?;

    let cypher = build_relationship_with_provenance_cypher(rel_type);

    let mut result = txn
        .execute(
            query(&cypher)
                .param("from_id", from_id)
                .param("to_id", to_id)
                .param("source_document_id", source_document_id)
                .param("extraction_run_id", extraction_run_id),
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Cypher failed for {rel_type} {from_id}->{to_id}: {e}"),
        })?;

    if result
        .next(&mut *txn)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Row fetch failed for {rel_type}: {e}"),
        })?
        .is_none()
    {
        return Err(AppError::Internal {
            message: format!(
                "Broken ID mapping: {rel_type} from '{from_id}' to '{to_id}' — \
                 MATCH found no nodes. Transaction will roll back."
            ),
        });
    }

    Ok(())
}

/// Create DERIVED_FROM relationships from provenance data in item_data.
///
/// For each item with a `provenance` array, finds the referenced entity
/// (typically a ComplaintAllegation matched by paragraph_number) and creates
/// a DERIVED_FROM relationship with `ref_type` and `quote_snippet` properties.
///
/// Stamps the three v5.1 provenance properties on every DERIVED_FROM edge
/// alongside the existing `ref_type` and `quote_snippet`. Source values:
/// - `source_document_id`: `doc_id` (the document being processed)
/// - `extraction_run_id`: `run-{i32}` derived from the ORIGINATING item's
///   `run_id` (the item that carries the provenance array — i.e., the
///   FROM-side of the DERIVED_FROM edge). That's the run that produced
///   this provenance link.
/// - `created_at`: Cypher `datetime()` at write time.
///
/// Returns the count of relationships created.
pub async fn create_provenance_relationships(
    txn: &mut neo4rs::Txn,
    items: &[ExtractionItemRecord],
    pg_to_neo4j: &HashMap<i32, String>,
    doc_id: &str,
) -> Result<usize, AppError> {
    // Build a lookup: paragraph key → extraction_item.id.
    // Accepts both `paragraph_number` (v2/v3) and `paragraph_ref` (v4),
    // and both string and integer shapes, so Harm provenance refs can
    // resolve regardless of which schema the ComplaintAllegation was
    // extracted under.
    let mut para_to_item_id: HashMap<String, i32> = HashMap::new();
    for item in items {
        let props = &item.item_data["properties"];
        let para = props["paragraph_number"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| props["paragraph_number"].as_i64().map(|n| n.to_string()))
            .or_else(|| props["paragraph_ref"].as_str().map(|s| s.to_string()))
            .or_else(|| props["paragraph_ref"].as_i64().map(|n| n.to_string()));
        if let Some(para) = para {
            para_to_item_id.insert(para, item.id);
        }
    }

    let mut count = 0usize;

    for item in items {
        let provenance = match item.item_data.get("provenance").and_then(|p| p.as_array()) {
            Some(arr) => arr,
            None => continue,
        };

        let from_neo = match pg_to_neo4j.get(&item.id) {
            Some(id) => id,
            None => continue,
        };

        // v5.1 provenance: every DERIVED_FROM edge from this item shares the
        // same source_document_id (`doc_id`) and extraction_run_id (the
        // originating item's run_id). Compute once per item and validate
        // once per item — per-edge validation would be redundant since the
        // values do not vary across `entry` iterations.
        let extraction_run_id = format!("run-{}", item.run_id);
        validate_relationship_provenance("DERIVED_FROM", doc_id, &extraction_run_id)?;

        for entry in provenance {
            let ref_type = entry["ref_type"].as_str().unwrap_or("paragraph");
            let ref_val = entry["ref"]
                .as_str()
                .map(|s| s.to_string())
                .or_else(|| entry["ref"].as_i64().map(|n| n.to_string()));

            let ref_val = match ref_val {
                Some(v) => v,
                None => continue,
            };

            let quote_snippet = entry["quote_snippet"].as_str().unwrap_or("");

            // Find the target item by paragraph number
            let target_item_id = match ref_type {
                "paragraph" => para_to_item_id.get(&ref_val),
                _ => continue, // Only paragraph references supported for now
            };

            let to_neo = match target_item_id.and_then(|id| pg_to_neo4j.get(id)) {
                Some(id) => id,
                None => {
                    tracing::debug!(
                        from = %from_neo, ref_type, ref_val = %ref_val,
                        "Provenance target not found — skipping DERIVED_FROM"
                    );
                    continue;
                }
            };

            // DERIVED_FROM uses its own Cypher (not the generic builder)
            // because it MERGEs on a composite key — `(rel_type, ref_type)` —
            // so different ref_types produce distinct edges between the same
            // pair of nodes. quote_snippet keeps its existing latest-wins
            // semantics (a description that may be refined on re-extraction);
            // the three v5.1 provenance properties use first-wins via
            // coalesce, matching the generic builder.
            let cypher = "MATCH (a {id: $from_id}), (b {id: $to_id}) \
                 MERGE (a)-[r:DERIVED_FROM {ref_type: $ref_type}]->(b) \
                 ON CREATE SET r.quote_snippet       = $snippet, \
                               r.source_document_id = $source_document_id, \
                               r.extraction_run_id  = $extraction_run_id, \
                               r.created_at         = datetime() \
                 ON MATCH SET  r.quote_snippet       = $snippet, \
                               r.source_document_id = coalesce(r.source_document_id, $source_document_id), \
                               r.extraction_run_id  = coalesce(r.extraction_run_id,  $extraction_run_id), \
                               r.created_at         = coalesce(r.created_at,         datetime()) \
                 RETURN b.id";

            let mut result = txn
                .execute(
                    query(cypher)
                        .param("from_id", from_neo.as_str())
                        .param("to_id", to_neo.as_str())
                        .param("ref_type", ref_type)
                        .param("snippet", quote_snippet)
                        .param("source_document_id", doc_id)
                        .param("extraction_run_id", extraction_run_id.as_str()),
                )
                .await
                .map_err(|e| AppError::Internal {
                    message: format!("Failed to create DERIVED_FROM {from_neo}->{to_neo}: {e}"),
                })?;

            match result.next(&mut *txn).await {
                Ok(Some(_)) => count += 1,
                Ok(None) => {
                    tracing::warn!(
                        from = %from_neo,
                        to = %to_neo,
                        ref_type = %ref_type,
                        "DERIVED_FROM MERGE returned no row — relationship not counted"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        from = %from_neo,
                        to = %to_neo,
                        ref_type = %ref_type,
                        error = %e,
                        "Failed to read DERIVED_FROM MERGE result — count may be incomplete"
                    );
                }
            }
        }
    }

    Ok(count)
}

/// Create CONTAINED_IN from all non-Document nodes to the Document.
///
/// `nodes_with_runs` pairs each Neo4j node id with the `run_id` of the
/// extraction run that produced the corresponding source entity. Per
/// v5.1 §5.4 every CONTAINED_IN edge is stamped with that run's id so a
/// later query can answer "which run added this entity to this document"
/// without consulting PostgreSQL.
///
/// `doc_id` is the PostgreSQL document id (not the Neo4j slug) — it
/// becomes the `source_document_id` provenance property. `doc_neo4j_id`
/// is the slug used as the Cypher MATCH endpoint.
///
/// ## Rust Learning: tuple parameter (`&[(String, i32)]`)
///
/// We pass a slice of tuples rather than a parallel HashMap because the
/// caller already iterates extraction items in order and the pairing is
/// 1:1 (after dedup). The tuple keeps the (node_id, run_id) association
/// explicit at the type level — it would be a mistake to thread two
/// separate slices and rely on index alignment.
pub async fn create_contained_in_relationships(
    txn: &mut neo4rs::Txn,
    nodes_with_runs: &[(String, i32)],
    doc_neo4j_id: &str,
    doc_id: &str,
) -> Result<usize, AppError> {
    for (node_id, run_id) in nodes_with_runs {
        // Pre-format the run id at the call site of the generic helper.
        // The helper itself is run-id-agnostic; the "run-{i32}" prefix
        // convention lives here and at the rel-write call site in
        // ingest.rs — both are the only two places that translate from
        // PG i32 to the prefixed string.
        let extraction_run_id = format!("run-{run_id}");
        create_ingest_relationship(
            txn,
            node_id,
            doc_neo4j_id,
            REL_CONTAINED_IN,
            doc_id,
            &extraction_run_id,
        )
        .await?;
    }
    Ok(nodes_with_runs.len())
}

/// Build the Cypher for an extracted cross-tier edge: an extraction node
/// (matched purely by `id`) to a canonical `:Element` node.
///
/// ## Why MATCH on both endpoints (not MERGE)
///
/// If either node is absent — the Allegation wasn't created this ingest, or
/// the canonical loader hasn't run to create the Element — the statement is a
/// no-op rather than creating a dangling bare node. The edge simply isn't
/// written until both real nodes exist.
///
/// `rel_type` is interpolated into the pattern (Cypher can't parameterize a
/// relationship type), so the caller MUST validate it is alphanumeric /
/// underscore — see [`write_cross_tier_relationship`]. `asserted_by_document`
/// tags the edge with the document whose Pass-2 extraction asserted it, which
/// is what [`delete_cross_tier_relationships_for_document`] keys cleanup on.
fn build_cross_tier_edge_cypher(rel_type: &str) -> String {
    format!(
        "MATCH (a {{id: $from_id}}) \
         MATCH (e:Element {{id: $to_id}}) \
         MERGE (a)-[r:{rel_type}]->(e) \
         ON CREATE SET r.asserted_by_document = $document_id, \
                       r.source_document_id   = $document_id, \
                       r.extraction_run_id    = $extraction_run_id, \
                       r.created_at           = datetime() \
         ON MATCH SET  r.asserted_by_document = $document_id, \
                       r.updated_at           = datetime()"
    )
}

/// True if `rel_type` is safe to interpolate into a Cypher relationship-type
/// position — non-empty and alphanumeric/underscore only. Cypher cannot
/// parameterize a relationship type, so [`write_cross_tier_relationship`]
/// interpolates it and must reject anything else (injection guard). Extracted
/// as a pure predicate so it can be unit-tested without a live transaction,
/// mirroring `validate_relationship_provenance` in this module.
fn cross_tier_rel_type_is_valid(rel_type: &str) -> bool {
    !rel_type.is_empty() && rel_type.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Write one extracted cross-tier relationship (e.g. `PROVES_ELEMENT`) into the
/// open transaction. Validates `rel_type` as a Cypher-injection guard, then
/// runs the MATCH-MATCH-MERGE from [`build_cross_tier_edge_cypher`].
///
/// `extraction_run_id` is pre-formatted by the caller (e.g. `"run-42"`),
/// matching the convention in [`create_contained_in_relationships`]. No row is
/// returned: a MATCH that finds no node is an intentional no-op, not an error.
pub async fn write_cross_tier_relationship(
    txn: &mut neo4rs::Txn,
    from_id: &str,
    to_id: &str,
    rel_type: &str,
    document_id: &str,
    extraction_run_id: &str,
) -> Result<(), AppError> {
    if !cross_tier_rel_type_is_valid(rel_type) {
        return Err(AppError::BadRequest {
            message: format!("Invalid cross-tier relationship type: '{rel_type}'"),
            details: serde_json::json!({ "rel_type": rel_type }),
        });
    }
    let cypher = build_cross_tier_edge_cypher(rel_type);
    txn.run(
        query(&cypher)
            .param("from_id", from_id)
            .param("to_id", to_id)
            .param("document_id", document_id)
            .param("extraction_run_id", extraction_run_id),
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to write {rel_type} {from_id}->{to_id}: {e}"),
    })?;
    Ok(())
}

/// Delete every cross-tier edge a document previously asserted in Neo4j,
/// matched purely on the `asserted_by_document` property.
///
/// Type-agnostic on purpose: `asserted_by_document` is set ONLY by
/// [`write_cross_tier_relationship`] (canonical-loader and standard ingest
/// edges never carry it), so matching on it alone reconciles *all* cross-tier
/// edge types this document wrote — today `PROVES_ELEMENT`, and any future
/// type — without burning a relationship-type literal into the query. This is
/// the graph-side complement to the Postgres reconciliation
/// (`delete_extracted_authored_relationships_for_document`): a re-process
/// clears the document's prior edges before re-asserting the current set.
pub async fn delete_cross_tier_relationships_for_document(
    txn: &mut neo4rs::Txn,
    document_id: &str,
) -> Result<(), AppError> {
    txn.run(
        query("MATCH ()-[r {asserted_by_document: $document_id}]->() DELETE r")
            .param("document_id", document_id),
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to delete prior cross-tier edges for {document_id}: {e}"),
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::pipeline_repository::ExtractionItemRecord;

    fn make_item(entity_type: &str, properties: serde_json::Value) -> ExtractionItemRecord {
        ExtractionItemRecord {
            id: 1,
            run_id: 1,
            document_id: "doc-awad-v-catholic-family-complaint-11-1-13".to_string(),
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
    fn test_stable_id_complaint_allegation_by_paragraph() {
        let item = make_item(
            "ComplaintAllegation",
            serde_json::json!({ "paragraph_number": "42", "summary": "test" }),
        );
        let id = stable_entity_id(&item, DOC_ID);
        assert!(id.starts_with("doc-awad-v-catholic-family-complaint-11-1-13:para:"));
        assert!(
            id.ends_with(":para:42"),
            "ID should end with :para:42, got: {}",
            id
        );
    }

    #[test]
    fn test_stable_id_complaint_allegation_numeric_paragraph() {
        // paragraph_number can be stored as a JSON number, not just string
        let item = make_item(
            "ComplaintAllegation",
            serde_json::json!({ "paragraph_number": 42 }),
        );
        let id = stable_entity_id(&item, DOC_ID);
        assert!(
            id.ends_with(":para:42"),
            "Numeric paragraph_number should produce same ID as string, got: {}",
            id
        );
    }

    #[test]
    fn test_stable_id_legal_count_by_number() {
        // New case-global format: `count-{N}`, no doc_slug prefix, matching
        // the canonical loader's `legal_count_entity_id`.
        let item = make_item(
            "LegalCount",
            serde_json::json!({ "count_number": 3, "legal_basis": "Breach of Contract" }),
        );
        let id = stable_entity_id(&item, DOC_ID);
        assert_eq!(id, "count-3", "numeric count_number must produce count-3");
    }

    #[test]
    fn test_stable_id_legal_count_numeric_one() {
        let item = make_item("LegalCount", serde_json::json!({ "count_number": 1 }));
        assert_eq!(stable_entity_id(&item, DOC_ID), "count-1");
    }

    #[test]
    fn test_stable_id_legal_count_string_number() {
        // count_number can arrive as a JSON string ("1") rather than a number;
        // it must still resolve to count-1 (the as_str branch).
        let item = make_item("LegalCount", serde_json::json!({ "count_number": "1" }));
        assert_eq!(
            stable_entity_id(&item, DOC_ID),
            "count-1",
            "string count_number must produce the same id as the numeric form"
        );
    }

    #[test]
    fn test_stable_id_legal_count_hash_fallback() {
        // No usable count_number: fall back to a content hash of legal_basis,
        // prefixed `count-hash-` (still case-global, no doc_slug).
        let item = make_item(
            "LegalCount",
            serde_json::json!({ "legal_basis": "Breach of Contract" }),
        );
        let id = stable_entity_id(&item, DOC_ID);
        let expected_hash = &format!("{:x}", Sha256::digest(b"Breach of Contract"))[..8];
        assert_eq!(
            id,
            format!("count-hash-{expected_hash}"),
            "missing count_number must hash legal_basis with a count-hash- prefix"
        );
        // Must not be doc-scoped, and must not look like a real count-{N}.
        assert!(
            !id.contains(':'),
            "fallback id must carry no doc_slug, got: {id}"
        );
        assert!(id.starts_with("count-hash-"));
    }

    #[test]
    fn test_stable_id_legal_count_resolves_across_documents() {
        // The whole point of this change (entity resolution): the same Count
        // cited in two different documents must produce ONE id, so ingest
        // MERGEs both onto the single canonical node instead of duplicating.
        let item = make_item("LegalCount", serde_json::json!({ "count_number": 1 }));
        let id_complaint = stable_entity_id(&item, "doc-awad-complaint");
        let id_motion = stable_entity_id(&item, "doc-awad-motion-to-compel");
        assert_eq!(id_complaint, "count-1");
        assert_eq!(
            id_complaint, id_motion,
            "same count_number in different documents must resolve to one id"
        );
    }

    #[test]
    fn test_stable_id_harm_by_content_hash() {
        let item = make_item(
            "Harm",
            serde_json::json!({
                "harm_type": "financial",
                "description": "Lost wages and benefits"
            }),
        );
        let id1 = stable_entity_id(&item, DOC_ID);
        // Run again — same inputs must produce same ID
        let id2 = stable_entity_id(&item, DOC_ID);
        assert_eq!(id1, id2, "Harm ID must be deterministic");
        assert!(
            id1.contains(":harm:"),
            "Harm ID must contain :harm: segment, got: {}",
            id1
        );
        // Hash segment must be 8 hex chars
        let hash_part = id1.split(":harm:").nth(1).unwrap_or("");
        assert_eq!(
            hash_part.len(),
            8,
            "Hash segment must be 8 hex chars, got: '{}'",
            hash_part
        );
    }

    #[test]
    fn test_stable_id_different_documents_differ() {
        let item = make_item(
            "ComplaintAllegation",
            serde_json::json!({ "paragraph_number": "42" }),
        );
        let id1 = stable_entity_id(&item, "doc-awad-complaint");
        let id2 = stable_entity_id(&item, "doc-different-complaint");
        assert_ne!(
            id1, id2,
            "Same paragraph in different documents must produce different IDs"
        );
    }

    #[test]
    fn test_stable_id_same_paragraph_same_document_same_id() {
        // This is the core idempotency guarantee.
        // The same entity extracted twice must produce the same ID.
        let item = make_item(
            "ComplaintAllegation",
            serde_json::json!({ "paragraph_number": "42", "summary": "Plaintiff was fired" }),
        );
        let id1 = stable_entity_id(&item, DOC_ID);
        let id2 = stable_entity_id(&item, DOC_ID);
        assert_eq!(
            id1, id2,
            "Same entity extracted twice must produce same ID (idempotency guarantee)"
        );
    }

    #[test]
    fn test_document_id_uses_doc_id_not_title() {
        // Verifies create_document_node generates its ID from doc_id directly
        // (no "doc-" re-prefix). Since doc_id itself starts with "doc-", the
        // resulting neo4j id still has the expected single prefix.
        // Regression guard for B6: previously this produced "doc-doc-...".
        let expected = slug(DOC_ID);
        assert!(!expected.is_empty());
        assert!(expected.starts_with("doc-"));
        assert!(
            !expected.starts_with("doc-doc-"),
            "neo4j Document id must not be double-prefixed; got: {expected}"
        );
        assert_eq!(expected, slug(DOC_ID));
    }

    #[test]
    fn test_slug_is_stable() {
        // slug() must be deterministic
        assert_eq!(slug("MARIE AWAD"), slug("marie awad"));
        assert_eq!(slug("Marie Awad"), "marie-awad");
        assert_eq!(slug("Catholic Family Services"), "catholic-family-services");
    }

    // ── P1 regression: v4 schema compatibility ────────────────────
    //
    // v4 ComplaintAllegation uses `paragraph_ref` and `allegation_text`
    // where v2/v3 used `paragraph_number` and `summary`. stable_entity_id
    // must read both vocabularies; otherwise every v4 allegation with
    // absent paragraph_number falls to the empty-string hash and they
    // all MERGE into one node.

    #[test]
    fn test_stable_id_v4_paragraph_ref_string() {
        // v4 ComplaintAllegation with paragraph_ref as string.
        let item = make_item(
            "ComplaintAllegation",
            serde_json::json!({ "paragraph_ref": "42", "allegation_text": "test" }),
        );
        let id = stable_entity_id(&item, DOC_ID);
        assert!(
            id.ends_with(":para:42"),
            "v4 paragraph_ref (string) must produce :para:42, got: {id}"
        );
    }

    #[test]
    fn test_stable_id_v4_paragraph_ref_numeric() {
        // v4 ComplaintAllegation with paragraph_ref as JSON number.
        let item = make_item(
            "ComplaintAllegation",
            serde_json::json!({ "paragraph_ref": 42, "allegation_text": "test" }),
        );
        let id = stable_entity_id(&item, DOC_ID);
        assert!(
            id.ends_with(":para:42"),
            "v4 paragraph_ref (numeric) must produce :para:42, got: {id}"
        );
    }

    #[test]
    fn test_stable_id_v2_paragraph_number_still_wins_over_v4() {
        // If both fields are present (unlikely in practice), prefer
        // paragraph_number so the v2/v3 id shape is preserved for
        // migration-era documents.
        let item = make_item(
            "ComplaintAllegation",
            serde_json::json!({
                "paragraph_number": "7",
                "paragraph_ref": "42",
            }),
        );
        let id = stable_entity_id(&item, DOC_ID);
        assert!(
            id.ends_with(":para:7"),
            "paragraph_number must take precedence over paragraph_ref, got: {id}"
        );
    }

    #[test]
    fn test_stable_id_v4_allegation_text_fallback_when_no_paragraph() {
        // Neither paragraph field present; fall back to allegation_text
        // hash rather than the empty-string hash. Two allegations with
        // different text must produce different ids.
        let a = make_item(
            "ComplaintAllegation",
            serde_json::json!({ "allegation_text": "Defendant fired plaintiff." }),
        );
        let b = make_item(
            "ComplaintAllegation",
            serde_json::json!({ "allegation_text": "Defendant withheld wages." }),
        );
        let id_a = stable_entity_id(&a, DOC_ID);
        let id_b = stable_entity_id(&b, DOC_ID);
        assert_ne!(
            id_a, id_b,
            "v4 allegations with different text must not collide via empty-hash fallback"
        );
        // And they should not be the empty-string hash sentinel.
        let empty_hash = format!(
            "{}:para:hash-{}",
            slug(DOC_ID),
            &format!("{:x}", Sha256::digest(b"".as_slice()))[..8]
        );
        assert_ne!(
            id_a, empty_hash,
            "allegation_text fallback produced empty-hash collision"
        );
    }

    #[test]
    fn test_stable_id_v2_summary_fallback_still_works() {
        // Regression: v2 items with only `summary` (no paragraph_number)
        // must still hash on summary, not fall through to allegation_text.
        let item = make_item(
            "ComplaintAllegation",
            serde_json::json!({ "summary": "v2 body text" }),
        );
        let id = stable_entity_id(&item, DOC_ID);
        let expected_hash = &format!("{:x}", Sha256::digest(b"v2 body text"))[..8];
        assert!(
            id.ends_with(&format!(":para:hash-{expected_hash}")),
            "v2 summary hash should drive the id; got {id}"
        );
    }

    // ── v5.1 relationship provenance ────────────────────────────────
    //
    // Per LEGAL_DATA_MODEL_v5_1 §5.4 every relationship written to Neo4j
    // carries source_document_id, extraction_run_id, and created_at.
    // These tests pin the contract at two layers:
    //   1. validate_relationship_provenance — the defensive guard that
    //      catches a future regression dropping a property at a call site.
    //   2. build_relationship_with_provenance_cypher — the Cypher template
    //      used by the generic writer. Asserting SET-clause content here
    //      catches a refactor that silently changes the write semantics.
    //
    // No async / Neo4j integration tests live in this module; the live
    // Cypher path is exercised by the DEV deploy verification step.

    #[test]
    fn validate_relationship_provenance_rejects_empty_source_document_id() {
        // Catches: a future refactor at any of the six caller sites that
        // accidentally passes "" for source_document_id (e.g., a logic
        // error in deriving doc_id) instead of producing a malformed edge.
        let result = validate_relationship_provenance("HAS_ELEMENT", "", "run-42");
        match result {
            Err(AppError::Internal { message }) => {
                assert!(
                    message.contains("source_document_id"),
                    "error message must name the missing property; got: {message}"
                );
                assert!(
                    message.contains("HAS_ELEMENT"),
                    "error message must name the relationship type; got: {message}"
                );
            }
            other => {
                panic!("expected AppError::Internal naming source_document_id; got: {other:?}")
            }
        }
    }

    #[test]
    fn validate_relationship_provenance_rejects_empty_extraction_run_id() {
        // Catches: a future refactor that drops the format!("run-{}", _)
        // at a call site, leaving extraction_run_id as the empty string.
        let result = validate_relationship_provenance("PROVES_ELEMENT", "doc-awad", "");
        match result {
            Err(AppError::Internal { message }) => {
                assert!(
                    message.contains("extraction_run_id"),
                    "error message must name the missing property; got: {message}"
                );
                assert!(
                    message.contains("PROVES_ELEMENT"),
                    "error message must name the relationship type; got: {message}"
                );
            }
            other => panic!("expected AppError::Internal naming extraction_run_id; got: {other:?}"),
        }
    }

    #[test]
    fn build_relationship_cypher_sets_source_document_id_on_create() {
        // Catches: a refactor that drops the source_document_id SET clause
        // (which would write edges without that property and silently
        // pass the validate step, since validate_* runs against the
        // input parameters, not the emitted Cypher).
        let cypher = build_relationship_with_provenance_cypher("HAS_ELEMENT");
        assert!(
            cypher.contains("ON CREATE SET"),
            "Cypher must contain an ON CREATE SET clause; got: {cypher}"
        );
        assert!(
            cypher.contains("r.source_document_id = $source_document_id"),
            "ON CREATE SET must assign r.source_document_id; got: {cypher}"
        );
    }

    #[test]
    fn build_relationship_cypher_sets_extraction_run_id_on_create() {
        // Catches: a refactor that drops the extraction_run_id SET clause
        // — same risk profile as the source_document_id test above.
        let cypher = build_relationship_with_provenance_cypher("PROVES_ELEMENT");
        assert!(
            cypher.contains("r.extraction_run_id = $extraction_run_id"),
            "ON CREATE SET must assign r.extraction_run_id; got: {cypher}"
        );
    }

    #[test]
    fn build_relationship_cypher_sets_created_at_on_create() {
        // Catches: a refactor that replaces datetime() with a Rust-side
        // timestamp string (which would risk timezone drift between the
        // backend host and the Neo4j server) or drops created_at entirely.
        let cypher = build_relationship_with_provenance_cypher("ABOUT");
        assert!(
            cypher.contains("r.created_at = datetime()"),
            "ON CREATE SET must assign r.created_at = datetime(); got: {cypher}"
        );
    }

    #[test]
    fn build_relationship_cypher_uses_coalesce_on_match() {
        // Catches: a refactor that switches ON MATCH from coalesce
        // (first-wins, the design decision in §5.4) to overwrite
        // (latest-wins). Latest-wins on ANY of the three properties
        // would make the provenance trio internally inconsistent —
        // created_at would no longer correspond to extraction_run_id.
        let cypher = build_relationship_with_provenance_cypher("CAUSED_BY");
        assert!(
            cypher.contains("ON MATCH SET"),
            "Cypher must contain an ON MATCH SET clause; got: {cypher}"
        );
        assert!(
            cypher.contains("coalesce(r.source_document_id, $source_document_id)"),
            "ON MATCH must coalesce r.source_document_id (first-wins); got: {cypher}"
        );
        assert!(
            cypher.contains("coalesce(r.extraction_run_id, $extraction_run_id)"),
            "ON MATCH must coalesce r.extraction_run_id (first-wins); got: {cypher}"
        );
        assert!(
            cypher.contains("coalesce(r.created_at, datetime())"),
            "ON MATCH must coalesce r.created_at (first-wins); got: {cypher}"
        );
    }

    // ── Cross-tier edge (PROVES_ELEMENT) ─────────────────────────────

    #[test]
    fn build_cross_tier_edge_cypher_matches_both_endpoints() {
        // Catches a refactor that switches either endpoint from MATCH to
        // MERGE — which would create dangling bare nodes when the Allegation
        // or canonical Element doesn't exist, instead of a no-op.
        let cypher = build_cross_tier_edge_cypher("PROVES_ELEMENT");
        assert!(
            cypher.contains("MATCH (a {id: $from_id})"),
            "from endpoint must be MATCHed by id; got: {cypher}"
        );
        assert!(
            cypher.contains("MATCH (e:Element {id: $to_id})"),
            "to endpoint must be MATCHed as :Element by id; got: {cypher}"
        );
        assert!(
            cypher.contains("MERGE (a)-[r:PROVES_ELEMENT]->(e)"),
            "rel type must be interpolated into the MERGE; got: {cypher}"
        );
    }

    #[test]
    fn build_cross_tier_edge_cypher_tags_asserting_document() {
        // The `asserted_by_document` property is what per-document cleanup
        // (delete_cross_tier_relationships_for_document) keys on — it must be
        // stamped on both ON CREATE and ON MATCH so a re-MERGE keeps it.
        let cypher = build_cross_tier_edge_cypher("PROVES_ELEMENT");
        assert!(cypher.contains("ON CREATE SET"), "got: {cypher}");
        assert!(cypher.contains("ON MATCH SET"), "got: {cypher}");
        assert_eq!(
            cypher
                .matches("r.asserted_by_document = $document_id")
                .count(),
            2,
            "asserted_by_document must be set on both branches; got: {cypher}"
        );
        assert!(
            cypher.contains("r.created_at") && cypher.contains("datetime()"),
            "ON CREATE must stamp created_at; got: {cypher}"
        );
    }

    #[test]
    fn build_cross_tier_edge_cypher_interpolates_rel_type() {
        // The type is interpolated (Cypher can't parameterize it), so a
        // different validated type flows through verbatim.
        let cypher = build_cross_tier_edge_cypher("CHARACTERIZES");
        assert!(
            cypher.contains("[r:CHARACTERIZES]->"),
            "rel type must be interpolated verbatim; got: {cypher}"
        );
    }

    #[test]
    fn cross_tier_rel_type_validator_accepts_safe_and_rejects_unsafe() {
        // Guards the Cypher interpolation in write_cross_tier_relationship:
        // only non-empty alphanumeric/underscore types may be interpolated.
        assert!(cross_tier_rel_type_is_valid("PROVES_ELEMENT"));
        assert!(cross_tier_rel_type_is_valid("CHARACTERIZES"));
        // Injection / malformed inputs must be rejected.
        assert!(!cross_tier_rel_type_is_valid(""), "empty must be rejected");
        assert!(
            !cross_tier_rel_type_is_valid("PROVES ELEMENT"),
            "whitespace must be rejected"
        );
        assert!(
            !cross_tier_rel_type_is_valid("PROVES_ELEMENT]->(x) DELETE r //"),
            "injection payload must be rejected"
        );
    }
}
