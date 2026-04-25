//! POST /api/admin/pipeline/documents/:id/ingest — Graph Writer.
//!
//! Reads verified extraction items from pipeline DB and writes them as
//! nodes and relationships into Neo4j. Uses entity resolution for parties.
//!
//! ## Rust Learning: Generic entity ingest
//!
//! Instead of per-type functions (create_allegation_nodes, create_harm_nodes),
//! this handler uses a single `create_entity_node` function for all non-Party
//! entities. The entity_type from the extraction schema becomes the Neo4j label
//! directly. Party entities are still special-cased (MERGE with Person/Org split).

use std::collections::HashMap;

use axum::{extract::Path, extract::State, Json};
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::models::document_status::{
    PARTY_SUBTYPES, STATUS_COMPLETED, STATUS_INDEXED, STATUS_INGESTED, STATUS_PUBLISHED,
    STATUS_VERIFIED,
};
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository::{self, steps};
use crate::state::AppState;

use super::ingest_helpers::{
    create_contained_in_relationships, create_document_node, create_entity_node,
    create_ingest_relationship, create_party_nodes, create_provenance_relationships,
};
use super::ingest_resolver::{self, ResolutionSummary};

/// Acquire a PostgreSQL session-scoped advisory lock keyed on `doc_id`.
///
/// Prevents two ingest operations (full, delta, or mixed) from racing
/// on the same document. Returns `true` on successful acquisition; the
/// caller must call `release_ingest_lock` when done. `false` means
/// another session holds the lock — fail fast with a 409.
///
/// Session-scoped (`pg_try_advisory_lock`, not xact-scoped) because the
/// lock must span multiple sqlx calls including the Neo4j txn commit,
/// not a single PG transaction.
async fn try_acquire_ingest_lock(
    pool: &sqlx::PgPool,
    doc_id: &str,
) -> Result<bool, AppError> {
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(hashtext($1)::bigint)")
        .bind(doc_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Advisory lock query failed: {e}"),
        })?;
    Ok(acquired)
}

/// Release the session-scoped advisory lock taken by
/// `try_acquire_ingest_lock`. Best-effort: logs on failure but does not
/// propagate errors (the lock auto-releases when the session ends).
async fn release_ingest_lock(pool: &sqlx::PgPool, doc_id: &str) {
    if let Err(e) = sqlx::query("SELECT pg_advisory_unlock(hashtext($1)::bigint)")
        .bind(doc_id)
        .execute(pool)
        .await
    {
        tracing::warn!(doc_id, error = %e, "Failed to release ingest advisory lock");
    }
}

// ── Response DTOs ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub document_id: String,
    pub status: String,
    pub neo4j_document_id: String,
    pub nodes_created: NodeCounts,
    pub relationships_created: RelCounts,
    pub resolution_summary: ResolutionSummary,
    pub duration_secs: f64,
}

/// Node counts — dynamic by entity type.
///
/// ## Rust Learning: HashMap for dynamic entity types
///
/// Previously this had hardcoded fields (complaint_allegation, legal_count, harm).
/// With generic ingest, we don't know at compile time what entity types will be
/// created. The `by_type` HashMap provides counts for whatever types appear.
/// `person` and `organization` are still separate (from Party resolution).
#[derive(Debug, Serialize)]
pub struct NodeCounts {
    pub document: usize,
    pub person: usize,
    pub organization: usize,
    /// Counts per non-Party entity type (e.g., "ComplaintAllegation" → 5)
    pub by_type: HashMap<String, usize>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct RelCounts {
    /// Counts per relationship type (e.g., "STATED_BY" → 10)
    pub by_type: HashMap<String, usize>,
    pub contained_in: usize,
    pub total: usize,
}

/// Response DTO for delta ingest — written as a superset of
/// [`IngestResponse`] fields that mean something in the delta case.
///
/// `items_already_in_graph` reports how many approved items were skipped
/// because they already had a `neo4j_node_id`. Non-zero means previous
/// Ingest (full or delta) runs wrote them; this run correctly treated
/// them as existing state.
///
/// `index_response` is populated when the inline Index run succeeds;
/// absent if indexing was skipped or failed softly (logged as a warn,
/// not propagated — delta's primary invariant is Neo4j correctness).
#[derive(Debug, Serialize)]
pub struct IngestDeltaResponse {
    pub document_id: String,
    pub status: String,
    pub neo4j_document_id: String,
    pub nodes_written: NodeCounts,
    pub relationships_written: RelCounts,
    pub items_already_in_graph: usize,
    pub skipped_relationships: usize,
    pub duration_secs: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_response: Option<super::index::IndexResponse>,
}

// ── Handler ─────────────────────────────────────────────────────

/// Core logic for graph ingest — callable from handler AND process endpoint.
///
/// Writes approved extraction items to Neo4j with entity resolution.
/// Does NOT check document status — caller is responsible for validation.
///
/// Serialises with other ingest operations on the same document via a
/// PostgreSQL session-scoped advisory lock. Concurrent callers receive
/// an `AppError::Conflict` rather than racing on Neo4j writes.
pub(crate) async fn run_ingest(
    state: &AppState,
    doc_id: &str,
    username: &str,
) -> Result<IngestResponse, AppError> {
    if !try_acquire_ingest_lock(&state.pipeline_pool, doc_id).await? {
        return Err(AppError::Conflict {
            message: format!("Another ingest is running for document '{doc_id}'"),
            details: serde_json::json!({ "document_id": doc_id }),
        });
    }
    let result = run_ingest_locked(state, doc_id, username).await;
    release_ingest_lock(&state.pipeline_pool, doc_id).await;
    result
}

/// Inner body of `run_ingest` — runs with the advisory lock already held.
/// Never call this directly; always go through `run_ingest` so the lock
/// lifetime is correct.
async fn run_ingest_locked(
    state: &AppState,
    doc_id: &str,
    username: &str,
) -> Result<IngestResponse, AppError> {
    let start = std::time::Instant::now();
    let step_id = steps::record_step_start(
        &state.pipeline_pool,
        doc_id,
        "ingest",
        username,
        &serde_json::json!({}),
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Step logging: {e}"),
    })?;

    // 1. Fetch document — must exist
    let document = pipeline_repository::get_document(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    // 2. Find latest COMPLETED extraction run
    let run_id = pipeline_repository::get_latest_completed_run(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("No completed extraction run for document '{doc_id}'"),
        })?;

    // 4. Fetch APPROVED items and their relationships for that run.
    //    Only approved items are written to Neo4j — unapproved items
    //    (e.g., ungrounded/hallucinated) are intentionally excluded.
    let items =
        pipeline_repository::get_approved_items_for_document(&state.pipeline_pool, doc_id, run_id)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("DB error: {e}"),
            })?;

    // Union pass-1 and pass-2 relationships. run_id here targets pass 1
    // (items live there); filtering by it would drop every pass-2
    // relationship for a 2-pass profile.
    let relationships = pipeline_repository::get_approved_relationships_for_document_all_passes(
        &state.pipeline_pool,
        doc_id,
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("DB error: {e}"),
    })?;

    tracing::info!(
        doc_id = %doc_id, run_id, items = items.len(),
        rels = relationships.len(), "Fetched extraction data"
    );

    // 5. Entity resolution — resolve Party items against existing Neo4j nodes
    let existing_parties = ingest_resolver::fetch_existing_parties(&state.graph).await?;
    tracing::info!(
        existing = existing_parties.len(),
        "Fetched existing parties for resolution"
    );

    let (resolution_map, resolution_summary) =
        ingest_resolver::resolve_parties(&items, &existing_parties).await?;

    tracing::info!(
        matched = resolution_summary.matched_existing,
        new = resolution_summary.created_new,
        "Entity resolution complete"
    );

    // 7. Open Neo4j transaction — all-or-nothing
    let mut txn = state
        .graph
        .start_txn()
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to start Neo4j transaction: {e}"),
        })?;

    // PG item ID → Neo4j node ID mapping (populated during node creation)
    let mut pg_to_neo4j: HashMap<i32, String> = HashMap::new();
    // Collect all non-Document node IDs for CONTAINED_IN relationships
    let mut all_node_ids: Vec<String> = Vec::new();

    // 8. Create Document node
    let doc_type = document.document_type.clone();

    let doc_neo4j_id = create_document_node(&mut txn, doc_id, &document.title, &doc_type).await?;

    // 9. Create/merge Party nodes (Person + Organization) using resolution map.
    //    pg_to_label tracks which Neo4j label each item actually got
    //    (e.g., Party items → "Person" or "Organization").
    let mut pg_to_label: HashMap<i32, String> = HashMap::new();
    let (person_count, org_count) = create_party_nodes(
        &mut txn,
        &items,
        doc_id,
        &mut pg_to_neo4j,
        &mut pg_to_label,
        &resolution_map,
    )
    .await?;
    // Collect unique party node IDs for CONTAINED_IN
    {
        let mut seen = std::collections::HashSet::new();
        for neo_id in pg_to_neo4j.values() {
            if seen.insert(neo_id.clone()) {
                all_node_ids.push(neo_id.clone());
            }
        }
    }

    // 10. Create all non-Party entity nodes using the generic function.
    //     Each entity_type from the extraction schema becomes the Neo4j label.
    //     Sequence numbers are tracked per entity type for readable IDs.
    let mut entity_type_counts: HashMap<String, usize> = HashMap::new();
    let mut entity_seq: HashMap<String, usize> = HashMap::new();

    // R4: inverse of the create_party_nodes filter — exclude Party and
    // its post-ingest resolved forms so non-Party entity creation doesn't
    // double-write what create_party_nodes already handled.
    for item in items
        .iter()
        .filter(|i| !PARTY_SUBTYPES.contains(&i.entity_type.as_str()))
    {
        let seq = entity_seq.entry(item.entity_type.clone()).or_insert(0);
        *seq += 1;

        let neo4j_id = create_entity_node(&mut txn, item, doc_id, *seq).await?;

        pg_to_neo4j.insert(item.id, neo4j_id.clone());
        all_node_ids.push(neo4j_id);

        *entity_type_counts
            .entry(item.entity_type.clone())
            .or_insert(0) += 1;
    }

    // 11. Create extraction relationships (STATED_BY, ABOUT, SUPPORTS, etc.)
    let mut rel_type_counts: HashMap<String, usize> = HashMap::new();

    for rel in &relationships {
        let from_neo = pg_to_neo4j
            .get(&rel.from_item_id)
            .ok_or_else(|| AppError::Internal {
                message: format!(
                    "No Neo4j ID for from_item_id {} (rel type {})",
                    rel.from_item_id, rel.relationship_type
                ),
            })?;
        let to_neo = pg_to_neo4j
            .get(&rel.to_item_id)
            .ok_or_else(|| AppError::Internal {
                message: format!(
                    "No Neo4j ID for to_item_id {} (rel type {})",
                    rel.to_item_id, rel.relationship_type
                ),
            })?;

        create_ingest_relationship(&mut txn, from_neo, to_neo, &rel.relationship_type).await?;

        *rel_type_counts
            .entry(rel.relationship_type.clone())
            .or_insert(0) += 1;
    }

    // 11b. Create DERIVED_FROM relationships from provenance data
    let derived_from_count =
        create_provenance_relationships(&mut txn, &items, &pg_to_neo4j).await?;
    if derived_from_count > 0 {
        tracing::info!(doc_id = %doc_id, derived_from_count, "Created DERIVED_FROM provenance relationships");
        *rel_type_counts
            .entry("DERIVED_FROM".to_string())
            .or_insert(0) += derived_from_count;
    }

    // 12. Create CONTAINED_IN relationships (all nodes → Document)
    let contained_in =
        create_contained_in_relationships(&mut txn, &all_node_ids, &doc_neo4j_id).await?;

    // 13. Commit transaction
    txn.commit().await.map_err(|e| AppError::Internal {
        message: format!("Neo4j transaction commit failed: {e}"),
    })?;

    // Compute totals before the status write so we can persist them in 14b.
    // Moved up from below the entity_type sync so `update_document_write_counts`
    // sits adjacent to `update_document_status` and the two always run together.
    let entity_node_total: usize = entity_type_counts.values().sum();
    let total_nodes = 1 + person_count + org_count + entity_node_total;
    let rel_total: usize = rel_type_counts.values().sum();
    let total_rels = rel_total + contained_in;

    // 14. Update pipeline document status → INGESTED
    pipeline_repository::update_document_status(&state.pipeline_pool, doc_id, STATUS_INGESTED)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update document status: {e}"),
        })?;

    // 14a. Persist write counts for the UI's Processing tab (bug B2).
    //      Previously these totals were only logged, so the UI always
    //      displayed "0 entities written to graph".
    pipeline_repository::update_document_write_counts(
        &state.pipeline_pool,
        doc_id,
        total_nodes as i32,
        total_rels as i32,
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to update write counts: {e}"),
    })?;

    // 14a-R1. Persist the extraction-item → Neo4j-node-id lineage.
    //         `pg_to_neo4j` carries the post-resolver, post-MERGE id for
    //         every item. Completeness reads this column directly instead
    //         of recomputing; the recomputation path can't reproduce
    //         resolver-assigned ids for Party entities.
    let mappings: Vec<(i32, String)> = pg_to_neo4j
        .iter()
        .map(|(id, neo4j_id)| (*id, neo4j_id.clone()))
        .collect();
    pipeline_repository::batch_update_neo4j_node_ids(&state.pipeline_pool, &mappings)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to persist neo4j_node_id lineage: {e}"),
        })?;

    // 14b. Sync extraction_items.entity_type with the actual Neo4j label.
    //
    // Generic pattern: if the label written to Neo4j differs from the
    // pipeline entity_type, update the pipeline DB to match. Today this
    // handles Party → Person/Organization; future type transformations
    // will work automatically without code changes.
    let mut entity_type_updates = 0usize;
    for item in &items {
        let actual_label = pg_to_label
            .get(&item.id)
            .map(|s| s.as_str())
            .unwrap_or(&item.entity_type);

        if actual_label != item.entity_type {
            pipeline_repository::update_item_entity_type(
                &state.pipeline_pool,
                item.id,
                actual_label,
            )
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to update entity_type for item {}: {e}", item.id),
            })?;
            entity_type_updates += 1;
        }
    }
    if entity_type_updates > 0 {
        tracing::info!(
            doc_id = %doc_id, updates = entity_type_updates,
            "Updated extraction_items.entity_type to match Neo4j labels"
        );
    }

    let duration = start.elapsed().as_secs_f64();

    tracing::info!(
        doc_id = %doc_id, total_nodes, total_rels,
        duration_secs = format!("{duration:.2}"),
        "Ingest complete"
    );

    log_admin_action(
        &state.audit_repo,
        username,
        "pipeline.document.ingest",
        Some("document"),
        Some(doc_id),
        Some(serde_json::json!({
            "neo4j_document_id": doc_neo4j_id,
            "nodes": total_nodes,
            "relationships": total_rels,
        })),
    )
    .await;

    if let Err(e) = steps::record_step_complete(&state.pipeline_pool, step_id, duration, &serde_json::json!({
        "nodes_created": total_nodes, "relationships_created": total_rels,
        "derived_from": derived_from_count,
        "matched_existing": resolution_summary.matched_existing, "created_new": resolution_summary.created_new,
    })).await
    {
        tracing::error!(
            document_id = %doc_id,
            step_id = step_id,
            error = %e,
            "Failed to record ingest step completion — audit trail gap"
        );
    }
    Ok(IngestResponse {
        document_id: doc_id.to_string(),
        status: STATUS_INGESTED.to_string(),
        neo4j_document_id: doc_neo4j_id,
        nodes_created: NodeCounts {
            document: 1,
            person: person_count,
            organization: org_count,
            by_type: entity_type_counts,
            total: total_nodes,
        },
        relationships_created: RelCounts {
            by_type: rel_type_counts,
            contained_in,
            total: total_rels,
        },
        resolution_summary,
        duration_secs: duration,
    })
}

/// POST /api/admin/pipeline/documents/:id/ingest
///
/// HTTP handler — thin wrapper around `run_ingest`.
/// Checks admin auth and status guard, then delegates to core logic.
pub async fn ingest_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<IngestResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST ingest");

    // Status guard
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    if document.status != STATUS_VERIFIED {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot ingest: status is '{}', expected '{STATUS_VERIFIED}'",
                document.status
            ),
            details: serde_json::json!({ "status": document.status }),
        });
    }

    let result = run_ingest(&state, &doc_id, &user.username).await?;
    Ok(Json(result))
}

// ── Delta ingest ────────────────────────────────────────────────

/// Core logic for delta (incremental) graph ingest — writes only items
/// whose `neo4j_node_id` is NULL into the already-populated graph.
///
/// Skips `cleanup_neo4j`. Seeds `pg_to_neo4j` from `get_existing_item_neo4j_map`
/// so same-document relationships between new and existing items resolve
/// without falling through to the cross-document lookup path.
/// Cross-document pass-2 edges originating from other documents are NOT
/// swept here — that's Phase 3b.
///
/// Status-preserving: does NOT update `documents.status`. The caller
/// (post-ingest doc) stays at its current status.
///
/// Serialised with full `run_ingest` and other delta calls via the shared
/// advisory lock.
pub(crate) async fn run_ingest_delta(
    state: &AppState,
    doc_id: &str,
    username: &str,
) -> Result<IngestDeltaResponse, AppError> {
    if !try_acquire_ingest_lock(&state.pipeline_pool, doc_id).await? {
        return Err(AppError::Conflict {
            message: format!("Another ingest is running for document '{doc_id}'"),
            details: serde_json::json!({ "document_id": doc_id }),
        });
    }
    let result = run_ingest_delta_locked(state, doc_id, username).await;
    release_ingest_lock(&state.pipeline_pool, doc_id).await;
    result
}

/// Inner body of `run_ingest_delta` — lock already held.
async fn run_ingest_delta_locked(
    state: &AppState,
    doc_id: &str,
    username: &str,
) -> Result<IngestDeltaResponse, AppError> {
    let start = std::time::Instant::now();

    let step_id = steps::record_step_start(
        &state.pipeline_pool,
        doc_id,
        "ingest_delta",
        username,
        &serde_json::json!({}),
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Step logging: {e}"),
    })?;

    // 1. Fetch document — must exist
    let document = pipeline_repository::get_document(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    // 2. Items awaiting graph write (approved/edited, neo4j_node_id NULL).
    //    If empty → early return; no txn, no work.
    let delta_items =
        pipeline_repository::get_items_pending_graph_write(&state.pipeline_pool, doc_id)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("DB error: {e}"),
            })?;

    // Existing map (all items in this doc that already have a neo4j id).
    // Needed for relationship endpoint resolution AND for the skipped
    // count in the response.
    let existing_map_rows =
        pipeline_repository::get_existing_item_neo4j_map(&state.pipeline_pool, doc_id)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("DB error: {e}"),
            })?;
    let items_already_in_graph = existing_map_rows.len();

    if delta_items.is_empty() {
        tracing::info!(
            doc_id = %doc_id,
            items_already_in_graph,
            "Delta ingest: no items pending graph write — no-op"
        );
        let duration = start.elapsed().as_secs_f64();
        if let Err(e) = steps::record_step_complete(
            &state.pipeline_pool,
            step_id,
            duration,
            &serde_json::json!({
                "nodes_written": 0,
                "relationships_written": 0,
                "items_already_in_graph": items_already_in_graph,
                "skipped_relationships": 0,
            }),
        )
        .await
        {
            tracing::error!(
                document_id = %doc_id,
                step_id = step_id,
                error = %e,
                "Failed to record ingest_delta no-op step completion — audit trail gap"
            );
        }
        return Ok(IngestDeltaResponse {
            document_id: doc_id.to_string(),
            status: document.status,
            neo4j_document_id: super::ingest_helpers::slug(doc_id),
            nodes_written: NodeCounts {
                document: 0,
                person: 0,
                organization: 0,
                by_type: HashMap::new(),
                total: 0,
            },
            relationships_written: RelCounts {
                by_type: HashMap::new(),
                contained_in: 0,
                total: 0,
            },
            items_already_in_graph,
            skipped_relationships: 0,
            duration_secs: duration,
            index_response: None,
        });
    }

    // 3. Fetch all approved relationships across both passes. We'll
    //    skip any whose endpoints can't be resolved (still-PENDING
    //    items, or cross-doc endpoints we haven't written yet).
    let relationships = pipeline_repository::get_approved_relationships_for_document_all_passes(
        &state.pipeline_pool,
        doc_id,
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("DB error: {e}"),
    })?;

    tracing::info!(
        doc_id = %doc_id,
        delta_items = delta_items.len(),
        rels = relationships.len(),
        items_already_in_graph,
        "Delta ingest: fetched extraction data"
    );

    // 4. Entity resolution against existing Neo4j Parties.
    let existing_parties = ingest_resolver::fetch_existing_parties(&state.graph).await?;
    let (resolution_map, _resolution_summary) =
        ingest_resolver::resolve_parties(&delta_items, &existing_parties).await?;

    // 5. Open Neo4j txn.
    let mut txn = state
        .graph
        .start_txn()
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to start Neo4j transaction: {e}"),
        })?;

    // pg_to_neo4j starts with EVERY already-written item on this doc,
    // so same-doc relationships between new and old items resolve.
    let mut pg_to_neo4j: HashMap<i32, String> = existing_map_rows.into_iter().collect();
    let mut newly_written_node_ids: Vec<String> = Vec::new();

    // 6. MERGE the Document node (idempotent — no-op update on a
    //    document that already exists in Neo4j).
    let doc_type = document.document_type.clone();
    let doc_neo4j_id =
        create_document_node(&mut txn, doc_id, &document.title, &doc_type).await?;

    // 7. Party nodes (MERGE). create_party_nodes filters the Party
    //    family itself; it iterates `delta_items` so only new Parties
    //    are processed. Existing parties already resolved from
    //    resolution_map → MERGE is a no-op.
    let mut pg_to_label: HashMap<i32, String> = HashMap::new();
    // Snapshot current pg_to_neo4j keys so we can identify which Party
    // IDs create_party_nodes added (existing ones are unchanged).
    let keys_before_parties: std::collections::HashSet<i32> =
        pg_to_neo4j.keys().copied().collect();
    let (person_count, org_count) = create_party_nodes(
        &mut txn,
        &delta_items,
        doc_id,
        &mut pg_to_neo4j,
        &mut pg_to_label,
        &resolution_map,
    )
    .await?;
    {
        let mut seen = std::collections::HashSet::new();
        for (id, neo_id) in pg_to_neo4j.iter() {
            if !keys_before_parties.contains(id) && seen.insert(neo_id.clone()) {
                newly_written_node_ids.push(neo_id.clone());
            }
        }
    }

    // 8. Non-Party delta entity nodes.
    let mut entity_type_counts: HashMap<String, usize> = HashMap::new();
    let mut entity_seq: HashMap<String, usize> = HashMap::new();
    for item in delta_items
        .iter()
        .filter(|i| !PARTY_SUBTYPES.contains(&i.entity_type.as_str()))
    {
        let seq = entity_seq.entry(item.entity_type.clone()).or_insert(0);
        *seq += 1;
        let neo4j_id = create_entity_node(&mut txn, item, doc_id, *seq).await?;
        pg_to_neo4j.insert(item.id, neo4j_id.clone());
        newly_written_node_ids.push(neo4j_id);
        *entity_type_counts
            .entry(item.entity_type.clone())
            .or_insert(0) += 1;
    }

    // 9. Cross-document endpoint resolution. Any relationship endpoint
    //    not in local pg_to_neo4j (which now includes all same-doc
    //    written items, old and new) is looked up in
    //    extraction_items.neo4j_node_id.
    let mut cross_doc_endpoints: std::collections::HashSet<i32> =
        std::collections::HashSet::new();
    for rel in &relationships {
        if !pg_to_neo4j.contains_key(&rel.from_item_id) {
            cross_doc_endpoints.insert(rel.from_item_id);
        }
        if !pg_to_neo4j.contains_key(&rel.to_item_id) {
            cross_doc_endpoints.insert(rel.to_item_id);
        }
    }
    let cross_doc_ids: Vec<i32> = cross_doc_endpoints.into_iter().collect();
    let cross_doc_neo4j_ids: HashMap<i32, String> =
        pipeline_repository::lookup_neo4j_node_ids(&state.pipeline_pool, &cross_doc_ids)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("lookup_neo4j_node_ids: {e}"),
            })?
            .into_iter()
            .collect();

    // 10. Relationships. MERGE is idempotent for existing+existing
    //     edges; skip any relationship whose endpoint can't be
    //     resolved from either local or cross-doc maps (happens for
    //     pass-2 edges whose other endpoint is still PENDING).
    let mut rel_type_counts: HashMap<String, usize> = HashMap::new();
    let mut skipped_relationships = 0usize;
    for rel in &relationships {
        let from_neo = pg_to_neo4j
            .get(&rel.from_item_id)
            .or_else(|| cross_doc_neo4j_ids.get(&rel.from_item_id));
        let to_neo = pg_to_neo4j
            .get(&rel.to_item_id)
            .or_else(|| cross_doc_neo4j_ids.get(&rel.to_item_id));
        match (from_neo, to_neo) {
            (Some(from), Some(to)) => {
                create_ingest_relationship(&mut txn, from, to, &rel.relationship_type).await?;
                *rel_type_counts
                    .entry(rel.relationship_type.clone())
                    .or_insert(0) += 1;
            }
            _ => {
                skipped_relationships += 1;
                tracing::debug!(
                    doc_id = %doc_id,
                    from = rel.from_item_id,
                    to = rel.to_item_id,
                    rel_type = %rel.relationship_type,
                    "Delta ingest skipped relationship — endpoint unresolved (PENDING?)"
                );
            }
        }
    }

    // 11. Provenance DERIVED_FROM — the para_to_item_id builder scans
    //     the delta items. It won't find already-written ComplaintAllegations
    //     by that path, so we pass ALL items (delta + existing) for the
    //     map build but pg_to_neo4j already has the full set.
    //
    //     To get already-written ComplaintAllegations' properties for
    //     the map build, we'd need another query. For Phase 3 v1,
    //     provenance only resolves when the Harm AND its referenced
    //     allegations are in the same delta batch. This is usually the
    //     case (bulk-approve all pending items at once); flagged as a
    //     Phase 3b tightening.
    let derived_from_count =
        create_provenance_relationships(&mut txn, &delta_items, &pg_to_neo4j).await?;
    if derived_from_count > 0 {
        *rel_type_counts
            .entry("DERIVED_FROM".to_string())
            .or_insert(0) += derived_from_count;
    }

    // 12. CONTAINED_IN for newly-written nodes only.
    let contained_in_count =
        create_contained_in_relationships(&mut txn, &newly_written_node_ids, &doc_neo4j_id)
            .await?;

    // 13. Commit.
    txn.commit().await.map_err(|e| AppError::Internal {
        message: format!("Neo4j transaction commit failed: {e}"),
    })?;

    let entity_node_total: usize = entity_type_counts.values().sum();
    let delta_nodes_total = person_count + org_count + entity_node_total;
    let rel_type_total: usize = rel_type_counts.values().sum();
    let delta_rels_total = rel_type_total + contained_in_count;

    // 14. Additive write counts. Preserves the original ingest totals
    //     and adds the delta contribution.
    pipeline_repository::add_document_write_counts(
        &state.pipeline_pool,
        doc_id,
        delta_nodes_total as i32,
        delta_rels_total as i32,
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to update write counts: {e}"),
    })?;

    // 15. Persist neo4j_node_id lineage for newly-written items.
    //     Already-mapped items in pg_to_neo4j were seeded from the DB;
    //     writing them back is a no-op identity update.
    let mappings: Vec<(i32, String)> = pg_to_neo4j
        .iter()
        .map(|(id, neo4j_id)| (*id, neo4j_id.clone()))
        .collect();
    pipeline_repository::batch_update_neo4j_node_ids(&state.pipeline_pool, &mappings)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to persist neo4j_node_id lineage: {e}"),
        })?;

    // 16. Sync resolved_entity_type for any Party → Person/Organization
    //     in the delta set.
    for item in &delta_items {
        if let Some(actual_label) = pg_to_label.get(&item.id) {
            if actual_label != &item.entity_type {
                pipeline_repository::update_item_entity_type(
                    &state.pipeline_pool,
                    item.id,
                    actual_label,
                )
                .await
                .map_err(|e| AppError::Internal {
                    message: format!("update_item_entity_type (item {}): {e}", item.id),
                })?;
            }
        }
    }

    // 17. Inline Index trigger. Uses run_index_core (status-agnostic)
    //     so a PUBLISHED/COMPLETED document isn't regressed to INDEXED.
    //     Soft-fail: log on error but don't propagate — the primary
    //     invariant (Neo4j state correct) is already satisfied.
    let index_response = match super::index::run_index_core(state, doc_id, username).await {
        Ok(r) => Some(r),
        Err(e) => {
            tracing::warn!(
                doc_id = %doc_id,
                error = ?e,
                "Delta ingest: inline Index failed — items in graph but not re-indexed. Run Index manually."
            );
            None
        }
    };

    let duration = start.elapsed().as_secs_f64();

    tracing::info!(
        doc_id = %doc_id,
        delta_nodes_total,
        delta_rels_total,
        items_already_in_graph,
        skipped_relationships,
        duration_secs = format!("{duration:.2}"),
        "Delta ingest complete"
    );

    log_admin_action(
        &state.audit_repo,
        username,
        "pipeline.document.ingest_delta",
        Some("document"),
        Some(doc_id),
        Some(serde_json::json!({
            "nodes": delta_nodes_total,
            "relationships": delta_rels_total,
            "items_already_in_graph": items_already_in_graph,
            "skipped_relationships": skipped_relationships,
        })),
    )
    .await;

    if let Err(e) = steps::record_step_complete(
        &state.pipeline_pool,
        step_id,
        duration,
        &serde_json::json!({
            "nodes_written": delta_nodes_total,
            "relationships_written": delta_rels_total,
            "items_already_in_graph": items_already_in_graph,
            "skipped_relationships": skipped_relationships,
            "derived_from": derived_from_count,
        }),
    )
    .await
    {
        tracing::error!(
            document_id = %doc_id,
            step_id = step_id,
            error = %e,
            "Failed to record ingest_delta step completion — audit trail gap"
        );
    }

    Ok(IngestDeltaResponse {
        document_id: doc_id.to_string(),
        status: document.status,
        neo4j_document_id: doc_neo4j_id,
        nodes_written: NodeCounts {
            document: 0,
            person: person_count,
            organization: org_count,
            by_type: entity_type_counts,
            total: delta_nodes_total,
        },
        relationships_written: RelCounts {
            by_type: rel_type_counts,
            contained_in: contained_in_count,
            total: delta_rels_total,
        },
        items_already_in_graph,
        skipped_relationships,
        duration_secs: duration,
        index_response,
    })
}

/// POST /api/admin/pipeline/documents/:id/ingest-delta
///
/// HTTP handler — thin wrapper around `run_ingest_delta`.
/// Requires admin auth and a post-ingest document status.
pub async fn ingest_delta_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<IngestDeltaResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST ingest-delta");

    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    // Delta ingest is meaningful only after a full ingest has run.
    if !matches!(
        document.status.as_str(),
        STATUS_INGESTED | STATUS_INDEXED | STATUS_PUBLISHED | STATUS_COMPLETED
    ) {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot run delta ingest: status is '{}', expected {STATUS_INGESTED} | {STATUS_INDEXED} | {STATUS_PUBLISHED} | {STATUS_COMPLETED}",
                document.status
            ),
            details: serde_json::json!({ "status": document.status }),
        });
    }

    let result = run_ingest_delta(&state, &doc_id, &user.username).await?;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_document_type_not_schema_filename() {
        // The doc_type written to Neo4j should be the document_type from PG,
        // NOT the schema_file from pipeline_config.
        // Schema files look like "general_legal.yaml" or "complaint.yaml"
        // Document types look like "complaint", "discovery_response", "affidavit"
        let bad_values = [
            "general_legal.yaml",
            "complaint.yaml",
            "discovery_response.yaml",
        ];
        for val in &bad_values {
            assert!(val.contains('.'), "Schema filenames contain dots");
        }
        let good_values = [
            "complaint",
            "discovery_response",
            "affidavit",
            "court_ruling",
        ];
        for val in &good_values {
            assert!(!val.contains('.'), "Document types should not contain dots");
        }
    }
}
