//! backend/src/pipeline/steps/ingest.rs
//!
//! Ingest step: writes approved extraction items into Neo4j as a
//! knowledge graph. Reuses the existing helpers in
//! `api::pipeline::ingest_helpers` and `api::pipeline::ingest_resolver`
//! unchanged — this file is the pipeline-framework wrapper.
//!
//! ## Rust Learning: idempotency — pragmatic compromise
//!
//! The canonical Neo4j idempotent-write pattern is MERGE with ON CREATE
//! / ON MATCH on every node and relationship, anchored on a stable
//! business key. This is also the canonical saga-step idempotency
//! pattern (Temporal, AWS, Azure all prescribe upserts over
//! delete-then-insert). The neo4j-labs llm-graph-builder reference
//! implementation does it this way via apoc.merge.node.
//!
//! colossus-legal's ingest_helpers currently uses CREATE (not MERGE)
//! for everything except Party entities. A naive retry would duplicate
//! nodes. Refactoring ingest_helpers to use MERGE is the correct fix
//! but is cross-cutting (it also changes the HTTP handler's behavior) —
//! it is tracked as dedicated follow-up **P-MERGE-refactor**.
//!
//! Until that refactor lands, this step uses a cleanup-then-write
//! idempotency model: call `cleanup_neo4j` first, then write fresh.
//! This produces correct results on retry but is wasteful compared
//! to MERGE. The cost is bounded (one DETACH DELETE pass per retry)
//! and acceptable at current scale.
//!
//! ## Rust Learning: saga compensation via on_cancel
//!
//! `on_cancel` calls `cleanup_neo4j` to reverse any partial writes that
//! happened before the cancel signal. This is the "compensating
//! transaction" half of the saga pattern. The step IS idempotent
//! (cleanup-then-write), so retry after cancel is also safe.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use colossus_pipeline::cancel::CancellationToken;
use colossus_pipeline::progress::ProgressReporter;
use colossus_pipeline::{Step, StepResult};

use crate::api::pipeline::ingest_helpers::{
    create_contained_in_relationships, create_document_node, create_entity_node,
    create_ingest_relationship, create_party_nodes, create_provenance_relationships,
};
use crate::api::pipeline::ingest_resolver;
use crate::models::document_status::{PARTY_SUBTYPES, STATUS_INGESTED};
use crate::pipeline::context::AppContext;
use crate::pipeline::steps::cleanup::{cleanup_neo4j, CleanupError};
use crate::pipeline::steps::index::Index;
use crate::pipeline::task::DocProcessing;
use crate::repositories::pipeline_repository;

/// Ingest step state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ingest {
    pub document_id: String,
}

/// Outcome of a successful pass through [`run_ingest`].
///
/// Consumed by:
/// - The legacy [`Ingest::execute`] thin wrapper — re-emits the
///   2-key audit JSON via `progress.set_step_result(...)` so
///   `pipeline_steps.result_summary` stays byte-identical to the
///   pre-refactor shape.
/// - The Restate workflow handler (`step_ingest`) — builds a
///   journal summary string from these counters.
///
/// Fixes bug B2 (`documents.entities_written` persistence) and
/// bug B3 (`pipeline_steps.result_summary` content).
#[derive(Debug, Clone, Default)]
pub struct IngestResult {
    /// Total Neo4j nodes written in this invocation (Document node +
    /// Party nodes + non-Party entity nodes).
    pub total_nodes: usize,
    /// Total Neo4j relationships written (extraction rels +
    /// DERIVED_FROM provenance + CONTAINED_IN).
    pub total_rels: usize,
}

// ─────────────────────────────────────────────────────────────────────────
// IngestError
// ─────────────────────────────────────────────────────────────────────────

/// Failure modes for the Ingest step.
///
/// Per-subsystem variants carry the `doc_id` being ingested and thread
/// the underlying error via `#[source]`. Display strings deliberately
/// omit `{source}` so log output does not duplicate the inner message
/// (Kazlauskas Guideline 6).
///
/// There is no `Postgres` variant: every PostgreSQL call routes through
/// `pipeline_repository::PipelineRepoError`, which collapses `sqlx::Error`
/// to a `String` at its `From` boundary. No code path in this step can
/// surface a raw `sqlx::Error`, so those failures land in `Helper` with a
/// `.to_string()` message — the debt is upstream, not here.
#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("Document '{doc_id}' not found")]
    DocumentNotFound { doc_id: String },

    #[error("No completed extraction run for document '{doc_id}'")]
    NoCompletedRun { doc_id: String },

    #[error("Pre-run cleanup failed for document '{doc_id}'")]
    Cleanup {
        doc_id: String,
        #[source]
        source: CleanupError,
    },

    #[error("Neo4j operation failed for document '{doc_id}'")]
    Neo4j {
        doc_id: String,
        #[source]
        source: neo4rs::Error,
    },

    /// Helper-origin failure. The underlying `AppError` /
    /// `PipelineRepoError` are stringly-typed, so we preserve only the
    /// message — not a source chain.
    #[error("Ingest helper failed for document '{doc_id}': {message}")]
    Helper { doc_id: String, message: String },
}

// ─────────────────────────────────────────────────────────────────────────
// Step impl
// ─────────────────────────────────────────────────────────────────────────

#[async_trait]
impl Step<DocProcessing> for Ingest {
    const DEFAULT_RETRY_LIMIT: i32 = 3;
    const DEFAULT_RETRY_DELAY_SECS: u64 = 10;
    const DEFAULT_TIMEOUT_SECS: Option<u64> = Some(300);

    /// Thin wrapper over [`run_ingest`] — the clean business core
    /// that the Restate workflow handler also calls.
    ///
    /// Adds on top of the core:
    /// 1. **Pre / post cancel checks** (legacy worker semantics).
    /// 2. **`progress.set_step_result(...)` audit JSON.** Re-emits
    ///    the 2-key shape (`entities_written`, `relationships_written`)
    ///    the pre-refactor body wrote inline so
    ///    `pipeline_steps.result_summary` stays byte-identical.
    /// 3. **FSM routing** to Index.
    ///
    /// The pre-run `cleanup_neo4j` call that used to live here moved
    /// INSIDE `run_ingest` so the Restate path's `ctx.run` replays
    /// also benefit from cleanup-then-write idempotency without the
    /// Restate handler having to call cleanup itself.
    async fn execute(
        self,
        db: &PgPool,
        context: &AppContext,
        cancel: &CancellationToken,
        progress: &ProgressReporter,
    ) -> Result<StepResult<DocProcessing>, Box<dyn Error + Send + Sync>> {
        let start = Instant::now();

        if cancel.is_cancelled().await {
            return Err("Cancelled before ingest".into());
        }

        let result = run_ingest(&self.document_id, db, context).await?;

        if cancel.is_cancelled().await {
            return Err("Cancelled after ingest".into());
        }

        let duration_secs = start.elapsed().as_secs_f64();
        tracing::info!(
            doc_id = %self.document_id,
            duration_secs,
            total_nodes = result.total_nodes,
            total_rels = result.total_rels,
            "Ingest step complete"
        );

        progress.set_step_result(serde_json::json!({
            "entities_written": result.total_nodes,
            "relationships_written": result.total_rels,
        }));

        Ok(StepResult::Next(DocProcessing::Index(Index {
            document_id: self.document_id,
        })))
    }

    async fn on_cancel(
        self,
        _db: &PgPool,
        context: &AppContext,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        cleanup_neo4j(&self.document_id, &context.graph)
            .await
            .map(|_| ())
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Core implementation (Restate-callable)
// ─────────────────────────────────────────────────────────────────────────

/// Run the Ingest step — write approved extraction items into Neo4j.
///
/// ## Cleanup-then-write idempotency
///
/// Calls [`cleanup_neo4j`] first to wipe any prior partial Neo4j
/// state for this `doc_id`, then performs the write inside a fresh
/// Neo4j transaction. This is the same idempotency model the legacy
/// worker used (the cleanup call previously lived in
/// `Step::execute`); moving it inside the core means Restate replay
/// — which re-executes the `ctx.run` closure body on workflow
/// recovery — gets cleanup-then-write for free, without forcing the
/// Restate handler to know about cleanup semantics.
///
/// The underlying `ingest_helpers` uses `CREATE` (not `MERGE`) for
/// everything except Party entities; a naive retry would duplicate
/// nodes. Cleanup-then-write is the bounded-cost workaround until
/// the cross-cutting **P-MERGE-refactor** lands. See module-level
/// docs for the rationale.
///
/// ## documents.status
///
/// Writes `STATUS_INGESTED` at the end of the function. Both legacy
/// and Restate paths see the canonical status surface via this core.
///
/// ## Cancellation
///
/// Does not poll a `CancellationToken`. The legacy worker wraps
/// `Step::execute` in `tokio::select!` with a `cancel_watcher`; the
/// Restate path kills the awaiting future via SDK abort.
///
/// Step numbering below mirrors the existing HTTP `ingest_handler`'s
/// step comments so anyone diff-reading between the two sees the
/// correspondence.
pub async fn run_ingest(
    document_id: &str,
    db: &PgPool,
    context: &AppContext,
) -> Result<IngestResult, IngestError> {
    let doc_id = document_id;

    // UI progress — step started.
    crate::pipeline::step_progress::write_start(db, context, doc_id, "ingest").await;

    // 0. Pre-run cleanup: wipe any prior partial Neo4j state for this
    //    doc_id before opening the write transaction. Makes retry safe
    //    even though the underlying helpers use CREATE rather than
    //    MERGE. Lives INSIDE the core (rather than the legacy
    //    `Step::execute` wrapper) so Restate replay also benefits.
    cleanup_neo4j(doc_id, &context.graph)
        .await
        .map_err(|source| IngestError::Cleanup {
            doc_id: doc_id.to_string(),
            source,
        })?;

    {
        // 1. Fetch document — must exist
        let document = pipeline_repository::get_document(db, doc_id)
            .await
            .map_err(|source| IngestError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("get_document: {source}"),
            })?
            .ok_or_else(|| IngestError::DocumentNotFound {
                doc_id: doc_id.to_string(),
            })?;

        // 2. HTTP handler checks `status == VERIFIED` here. The pipeline
        //    FSM enforces order via validate_transition, so we skip the
        //    double-gate.

        // 3. Find latest COMPLETED extraction run
        let run_id = pipeline_repository::get_latest_completed_run(db, doc_id)
            .await
            .map_err(|source| IngestError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("get_latest_completed_run: {source}"),
            })?
            .ok_or_else(|| IngestError::NoCompletedRun {
                doc_id: doc_id.to_string(),
            })?;

        // 4. Fetch approved items and relationships
        let items = pipeline_repository::get_approved_items_for_document(db, doc_id, run_id)
            .await
            .map_err(|source| IngestError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("get_approved_items: {source}"),
            })?;

        // Union pass-1 and pass-2 relationships. The run_id above is
        // scoped to pass 1 (that's where items live), so filtering
        // relationships by it would drop every pass-2 relationship on
        // the floor — breaking Ingest the first time a 2-pass profile
        // reaches this step.
        let relationships =
            pipeline_repository::get_approved_relationships_for_document_all_passes(db, doc_id)
                .await
                .map_err(|source| IngestError::Helper {
                    doc_id: doc_id.to_string(),
                    message: format!("get_approved_relationships_all_passes: {source}"),
                })?;

        tracing::info!(
            doc_id = %doc_id,
            run_id,
            items = items.len(),
            rels = relationships.len(),
            "Ingest: fetched extraction data"
        );

        // 5. Entity resolution
        let existing_parties = ingest_resolver::fetch_existing_parties(&context.graph)
            .await
            .map_err(|e| IngestError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("fetch_existing_parties: {e:?}"),
            })?;

        let (resolution_map, _resolution_summary) =
            ingest_resolver::resolve_parties(&items, &existing_parties)
                .await
                .map_err(|e| IngestError::Helper {
                    doc_id: doc_id.to_string(),
                    message: format!("resolve_parties: {e:?}"),
                })?;

        // 6. Open Neo4j transaction — all-or-nothing
        let mut txn = context
            .graph
            .start_txn()
            .await
            .map_err(|source| IngestError::Neo4j {
                doc_id: doc_id.to_string(),
                source,
            })?;

        // PG item ID → Neo4j node ID / label
        let mut pg_to_neo4j: HashMap<i32, String> = HashMap::new();
        let mut pg_to_label: HashMap<i32, String> = HashMap::new();

        // 7. Create Document node
        let doc_neo4j_id =
            create_document_node(&mut txn, doc_id, &document.title, &document.document_type)
                .await
                .map_err(|e| IngestError::Helper {
                    doc_id: doc_id.to_string(),
                    message: format!("create_document_node: {e:?}"),
                })?;

        // 8. Create Party nodes
        let (person_count, org_count) = create_party_nodes(
            &mut txn,
            &items,
            doc_id,
            &mut pg_to_neo4j,
            &mut pg_to_label,
            &resolution_map,
        )
        .await
        .map_err(|e| IngestError::Helper {
            doc_id: doc_id.to_string(),
            message: format!("create_party_nodes: {e:?}"),
        })?;

        // 9. Create non-Party entity nodes
        let mut entity_type_counts: HashMap<String, usize> = HashMap::new();
        let mut entity_seq: HashMap<String, usize> = HashMap::new();

        // R4: inverse of the create_party_nodes filter — exclude Party
        // and its post-ingest resolved forms so non-Party entity creation
        // doesn't double-write what create_party_nodes already handled.
        for item in items
            .iter()
            .filter(|i| !PARTY_SUBTYPES.contains(&i.entity_type.as_str()))
        {
            let seq = entity_seq.entry(item.entity_type.clone()).or_insert(0);
            *seq += 1;

            let neo4j_id = create_entity_node(&mut txn, item, doc_id, *seq)
                .await
                .map_err(|e| IngestError::Helper {
                    doc_id: doc_id.to_string(),
                    message: format!("create_entity_node: {e:?}"),
                })?;

            pg_to_neo4j.insert(item.id, neo4j_id.clone());
            *entity_type_counts
                .entry(item.entity_type.clone())
                .or_insert(0) += 1;
        }

        // 9b. Pair each Neo4j node id with the originating extraction
        //     item's run_id for the v5.1 CONTAINED_IN writer (per §5.4).
        //     Iterate items (not pg_to_neo4j.values()) because items
        //     carry run_id; pg_to_neo4j does not. Dedup on neo4j_id so
        //     Party MERGE deduplication doesn't double-emit
        //     CONTAINED_IN. First-seen run_id wins on dedup; all Party
        //     items in this pipeline-step batch share the same Pass-1
        //     run_id, so the choice is operationally inert.
        let mut all_nodes_with_runs: Vec<(String, i32)> = Vec::new();
        {
            let mut seen: HashSet<String> = HashSet::new();
            for item in &items {
                if let Some(neo_id) = pg_to_neo4j.get(&item.id) {
                    if seen.insert(neo_id.clone()) {
                        all_nodes_with_runs.push((neo_id.clone(), item.run_id));
                    }
                }
            }
        }

        // 10a. Resolve cross-document relationship endpoints.
        //      Pass 2 may emit relationships whose `from_item_id` /
        //      `to_item_id` reference items owned by a DIFFERENT
        //      document (e.g., a discovery response's CORROBORATES
        //      edge into a complaint's ComplaintAllegation). Those
        //      items are not in the local `items` vec, so
        //      `pg_to_neo4j` — built above from locals only —
        //      wouldn't resolve them. Look up their stored
        //      `extraction_items.neo4j_node_id` (populated by their
        //      own source-doc Ingest) and keep the results in a
        //      SEPARATE map: merging into `pg_to_neo4j` would cause
        //      `batch_update_neo4j_node_ids` to re-write the same
        //      values and `all_node_ids` / CONTAINED_IN to
        //      incorrectly attach cross-doc nodes to this document.
        let mut cross_doc_endpoints: HashSet<i32> = HashSet::new();
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
            pipeline_repository::lookup_neo4j_node_ids(db, &cross_doc_ids)
                .await
                .map_err(|source| IngestError::Helper {
                    doc_id: doc_id.to_string(),
                    message: format!("lookup_neo4j_node_ids: {source}"),
                })?
                .into_iter()
                .collect();

        // Look up source document ids for any endpoint we still can't
        // resolve — the error message names the owning document so
        // operators can distinguish "dangling reference" from
        // "cross-doc target not yet ingested."
        let unresolved_ids: Vec<i32> = cross_doc_ids
            .iter()
            .copied()
            .filter(|id| !cross_doc_neo4j_ids.contains_key(id))
            .collect();
        let unresolved_doc_ids: HashMap<i32, String> =
            pipeline_repository::lookup_item_document_ids(db, &unresolved_ids)
                .await
                .map_err(|source| IngestError::Helper {
                    doc_id: doc_id.to_string(),
                    message: format!("lookup_item_document_ids: {source}"),
                })?
                .into_iter()
                .collect();

        // Small helper to produce the enriched error suffix at the
        // unresolved-endpoint failure point. Closure rather than fn so
        // it can capture the lookup maps by reference.
        let describe_missing = |item_id: i32| -> String {
            match unresolved_doc_ids.get(&item_id) {
                Some(src_doc) if src_doc == doc_id => {
                    " [owned by this document, neo4j_node_id missing]".to_string()
                }
                Some(src_doc) => format!(
                    " [owned by document '{src_doc}', neo4j_node_id missing — source Ingest may have failed]"
                ),
                None => " [item not found in extraction_items — dangling reference]".to_string(),
            }
        };

        // 10b. Create extraction relationships
        let mut rel_type_counts: HashMap<String, usize> = HashMap::new();

        for rel in &relationships {
            let from_neo = pg_to_neo4j
                .get(&rel.from_item_id)
                .or_else(|| cross_doc_neo4j_ids.get(&rel.from_item_id))
                .ok_or_else(|| IngestError::Helper {
                    doc_id: doc_id.to_string(),
                    message: format!(
                        "No Neo4j ID for from_item_id {} (rel type {}){}",
                        rel.from_item_id,
                        rel.relationship_type,
                        describe_missing(rel.from_item_id)
                    ),
                })?;
            let to_neo = pg_to_neo4j
                .get(&rel.to_item_id)
                .or_else(|| cross_doc_neo4j_ids.get(&rel.to_item_id))
                .ok_or_else(|| IngestError::Helper {
                    doc_id: doc_id.to_string(),
                    message: format!(
                        "No Neo4j ID for to_item_id {} (rel type {}){}",
                        rel.to_item_id,
                        rel.relationship_type,
                        describe_missing(rel.to_item_id)
                    ),
                })?;

            // v5.1 §5.4: per-edge `extraction_run_id` is the relationship
            // row's own `run_id` (Pass-1 rels carry the Pass-1 id; Pass-2
            // rels carry the Pass-2 id). Pre-format the prefix here at
            // the call site — the writer is run-id-agnostic.
            let extraction_run_id = format!("run-{}", rel.run_id);
            create_ingest_relationship(
                &mut txn,
                from_neo,
                to_neo,
                &rel.relationship_type,
                doc_id,
                &extraction_run_id,
            )
            .await
            .map_err(|e| IngestError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("create_ingest_relationship: {e:?}"),
            })?;

            *rel_type_counts
                .entry(rel.relationship_type.clone())
                .or_insert(0) += 1;
        }

        // 11. DERIVED_FROM relationships from provenance
        let derived_from_count =
            create_provenance_relationships(&mut txn, &items, &pg_to_neo4j, doc_id)
                .await
                .map_err(|e| IngestError::Helper {
                    doc_id: doc_id.to_string(),
                    message: format!("create_provenance_relationships: {e:?}"),
                })?;

        // 12. CONTAINED_IN relationships
        let contained_in_count = create_contained_in_relationships(
            &mut txn,
            &all_nodes_with_runs,
            &doc_neo4j_id,
            doc_id,
        )
        .await
        .map_err(|e| IngestError::Helper {
            doc_id: doc_id.to_string(),
            message: format!("create_contained_in_relationships: {e:?}"),
        })?;

        // 13. Commit Neo4j txn
        txn.commit().await.map_err(|source| IngestError::Neo4j {
            doc_id: doc_id.to_string(),
            source,
        })?;

        // 14. Legacy status write.
        //
        // NOTE: transitional. Frontend, state_machine.rs, delete.rs,
        // graph_validation.rs key off documents.status = 'INGESTED'.
        // The pipeline framework's own pipeline_jobs.status is the
        // canonical step-status source. Phase 5 decides the fate of
        // documents.status at the HTTP/UI boundary.
        pipeline_repository::update_document_status(db, doc_id, STATUS_INGESTED)
            .await
            .map_err(|source| IngestError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("update_document_status: {source}"),
            })?;

        let total_nodes = 1 + person_count + org_count + entity_type_counts.values().sum::<usize>();
        let total_rels =
            rel_type_counts.values().sum::<usize>() + contained_in_count + derived_from_count;

        // 14b. Persist write counts for the UI's Processing tab (bug B2).
        //      Previously these totals were only logged.
        pipeline_repository::update_document_write_counts(
            db,
            doc_id,
            total_nodes as i32,
            total_rels as i32,
        )
        .await
        .map_err(|source| IngestError::Helper {
            doc_id: doc_id.to_string(),
            message: format!("update_document_write_counts: {source}"),
        })?;

        // 14c. R1: persist the extraction-item → Neo4j-node-id lineage.
        //      `pg_to_neo4j` carries the post-resolver, post-MERGE id for
        //      every item — including Party entities matched to
        //      pre-existing shared nodes. Completeness reads this column
        //      directly; without it, resolver-matched Parties surface as
        //      false-positive "missing" on verification.
        let mappings: Vec<(i32, String)> = pg_to_neo4j
            .iter()
            .map(|(id, neo4j_id)| (*id, neo4j_id.clone()))
            .collect();
        pipeline_repository::batch_update_neo4j_node_ids(db, &mappings)
            .await
            .map_err(|source| IngestError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("batch_update_neo4j_node_ids: {source}"),
            })?;

        // 15. Sync entity_type for Party → Person/Organization
        let mut entity_type_updates = 0usize;
        for item in &items {
            let actual_label = pg_to_label
                .get(&item.id)
                .map(|s| s.as_str())
                .unwrap_or(&item.entity_type);

            if actual_label != item.entity_type {
                pipeline_repository::update_item_entity_type(db, item.id, actual_label)
                    .await
                    .map_err(|source| IngestError::Helper {
                        doc_id: doc_id.to_string(),
                        message: format!("update_item_entity_type: {source}"),
                    })?;
                entity_type_updates += 1;
            }
        }

        tracing::info!(
            doc_id = %doc_id,
            neo4j_doc_id = %doc_neo4j_id,
            total_nodes,
            total_rels,
            person_count,
            org_count,
            derived_from_count,
            contained_in_count,
            entity_type_updates,
            ?entity_type_counts,
            ?rel_type_counts,
            "Ingest write complete"
        );

        // UI progress — step complete.
        crate::pipeline::step_progress::write_end(db, context, doc_id, "ingest").await;

        Ok(IngestResult {
            total_nodes,
            total_rels,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const UNIQUE_INNER: &str = "UNIQUE_INGEST_INNER_ERROR";

    fn dummy_neo4j_err() -> neo4rs::Error {
        neo4rs::Error::AuthenticationError(UNIQUE_INNER.to_string())
    }

    #[test]
    fn ingest_error_neo4j_display_excludes_source_text() {
        let err = IngestError::Neo4j {
            doc_id: "doc-42".to_string(),
            source: dummy_neo4j_err(),
        };
        let display = format!("{err}");
        assert!(display.contains("doc-42"), "got: {display}");
        assert!(
            !display.contains(UNIQUE_INNER),
            "Display must not duplicate inner source (Kazlauskas 6); got: {display}"
        );
    }

    #[test]
    fn ingest_error_document_not_found_display_contains_doc_id() {
        let err = IngestError::DocumentNotFound {
            doc_id: "missing-doc-99".to_string(),
        };
        assert!(format!("{err}").contains("missing-doc-99"));
    }
}
