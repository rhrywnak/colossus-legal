//! backend/src/pipeline/steps/completeness.rs
//!
//! Completeness step: verifies that the pipeline's PostgreSQL state,
//! Neo4j graph, and Qdrant vector store agree on counts. This is the
//! terminal step of the DocProcessing FSM — if it passes, the document
//! status moves to PUBLISHED and the pipeline job is Done.
//!
//! ## Rust Learning: transitional status write
//!
//! Tracker v1_5 says Completeness sets `DOC_STATUS_COMPLETED` — a
//! constant whose eventual value will be `"COMPLETED"`, per the planned
//! 5-state lifecycle simplification (PIPELINE_SIMPLIFICATION_TRACKER_v1
//! PS-B8). That migration has not happened yet. The current system
//! still uses the 8-state legacy lifecycle ending in `"PUBLISHED"`,
//! which the frontend, `state_machine`, `delete`, and the HTTP
//! completeness handler all key off of.
//!
//! If we wrote `"COMPLETED"` here today, the frontend would not
//! recognise the state, `state_machine` would return no actions, and
//! the Quality Check stage indicator would confuse. So this step
//! writes the same `STATUS_PUBLISHED` that the HTTP handler writes —
//! exactly the same transitional pattern as P4-5's `STATUS_INGESTED`
//! and P4-6's `STATUS_INDEXED` writes. Phase 5's lifecycle migration
//! owns the rename across all consumers in one coordinated change.
//!
//! ## Rust Learning: check-fail IS step-fail
//!
//! The HTTP completeness endpoint returns 200 OK with pass/fail
//! details in the body, because it's a diagnostic endpoint for a
//! human admin to inspect. The pipeline step cannot do that — the FSM
//! expects success or error. So any `"fail"` status in the check list
//! causes the step to return `Err(CompletenessError::ChecksFailed {
//! ... })`, which the framework records in
//! `pipeline_jobs.error_message`. `"warn"` status (only
//! `orphaned_nodes` uses it) does NOT trigger step failure.
//!
//! ## Rust Learning: read-only step, no-op on_cancel
//!
//! Completeness reads Neo4j + Qdrant + Postgres (counts only), then
//! writes exactly one row to Postgres (the status update). There's no
//! partial state to roll back on cancel — a cancel mid-query just
//! drops the read; a cancel pre-status-write leaves the document at
//! `INDEXED` (correct resume point); a cancel post-status-write
//! leaves it at `PUBLISHED` (also fine, the work succeeded). No
//! cleanup needed.

use std::collections::HashMap;
use std::error::Error;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use colossus_pipeline::cancel::CancellationToken;
use colossus_pipeline::progress::ProgressReporter;
use colossus_pipeline::{Step, StepResult};

use crate::api::pipeline::completeness::{compare_counts, CompareInput, CompletenessCheck};
use crate::api::pipeline::completeness_helpers::{
    count_neo4j_nodes_by_graph, count_neo4j_relationships_by_graph, find_orphaned_nodes_by_graph,
};
use crate::models::document_status::STATUS_PUBLISHED;
use crate::pipeline::constants::QDRANT_DOCUMENT_ID_FIELD;
use crate::pipeline::context::AppContext;
use crate::pipeline::task::DocProcessing;
use crate::repositories::pipeline_repository::{self, steps};
use crate::services::qdrant_service;

/// Completeness step state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Completeness {
    pub document_id: String,
}

/// Counts returned by [`Completeness::run_completeness`] for the
/// `pipeline_steps.result_summary` JSON (bug B3). Only populated when all
/// checks pass — failures return `Err(ChecksFailed)` before this is built.
#[derive(Debug)]
struct CompletenessStats {
    checks_passed: usize,
    checks_total: usize,
    warnings: usize,
    neo4j_total_nodes: usize,
    neo4j_total_rels: usize,
    qdrant_count: usize,
    orphaned_count: usize,
}

// ─────────────────────────────────────────────────────────────────────────
// CompletenessError
// ─────────────────────────────────────────────────────────────────────────

/// Failure modes for the Completeness step.
///
/// `ChecksFailed` deliberately embeds all the check details in its
/// Display text so `pipeline_jobs.error_message` gets actionable info
/// without requiring a separate GET to `/completeness`.
///
/// Other variants follow the Kazlauskas G6 discipline: `#[source]` where
/// the inner type is structurally useful, Display strings excluding
/// `{source}` text.
#[derive(Debug, thiserror::Error)]
pub enum CompletenessError {
    #[error("Document '{doc_id}' not found")]
    DocumentNotFound { doc_id: String },

    #[error("No completed extraction run for document '{doc_id}'")]
    NoCompletedRun { doc_id: String },

    #[error("Neo4j query failed for document '{doc_id}'")]
    Neo4j {
        doc_id: String,
        #[source]
        source: neo4rs::Error,
    },

    #[error("Qdrant query failed for document '{doc_id}': {message}")]
    Qdrant { doc_id: String, message: String },

    #[error(
        "Completeness checks failed for document '{doc_id}': {failed_count} of {total_count} checks failed. Details: {details}"
    )]
    ChecksFailed {
        doc_id: String,
        failed_count: usize,
        total_count: usize,
        details: String,
    },

    #[error("Helper failed for document '{doc_id}': {message}")]
    Helper { doc_id: String, message: String },
}

// ─────────────────────────────────────────────────────────────────────────
// Step impl
// ─────────────────────────────────────────────────────────────────────────

#[async_trait]
impl Step<DocProcessing> for Completeness {
    const DEFAULT_RETRY_LIMIT: i32 = 3;
    const DEFAULT_RETRY_DELAY_SECS: u64 = 10;
    const DEFAULT_TIMEOUT_SECS: Option<u64> = Some(60);

    async fn execute(
        self,
        db: &PgPool,
        context: &AppContext,
        cancel: &CancellationToken,
        _progress: &ProgressReporter,
    ) -> Result<StepResult<DocProcessing>, Box<dyn Error + Send + Sync>> {
        let start = Instant::now();
        let doc_id = self.document_id.clone();

        if cancel.is_cancelled().await {
            return Err("Cancelled before completeness check".into());
        }

        let step_id = steps::record_step_start(
            db,
            &doc_id,
            "completeness",
            "pipeline",
            &serde_json::json!({}),
        )
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(
                doc_id = %doc_id, error = %e,
                "Completeness: record_step_start failed (non-fatal)"
            );
            0
        });

        let stats = self.run_completeness(db, context, &doc_id).await?;

        let duration_secs = start.elapsed().as_secs_f64();
        tracing::info!(
            doc_id = %doc_id,
            duration_secs,
            "Completeness step complete — document PUBLISHED"
        );

        if step_id != 0 {
            let summary = serde_json::json!({
                "checks_passed": stats.checks_passed,
                "checks_total": stats.checks_total,
                "warnings": stats.warnings,
                "neo4j_total_nodes": stats.neo4j_total_nodes,
                "neo4j_total_rels": stats.neo4j_total_rels,
                "qdrant_count": stats.qdrant_count,
                "orphaned_count": stats.orphaned_count,
            });
            if let Err(e) = steps::record_step_complete(db, step_id, duration_secs, &summary).await
            {
                tracing::warn!(
                    doc_id = %doc_id, step_id, error = %e,
                    "Completeness: record_step_complete failed (non-fatal)"
                );
            }
        }

        Ok(StepResult::Done)
    }

    // on_cancel: read-only step, no partial state → trait-default no-op.
    // on_delete: trait default (Task::on_delete_current handles via
    // cleanup_all).
}

impl Completeness {
    /// Internal: fetch counts from all three stores, compare, and finalise.
    /// Called from [`Step::execute`].
    async fn run_completeness(
        &self,
        db: &PgPool,
        context: &AppContext,
        doc_id: &str,
    ) -> Result<CompletenessStats, CompletenessError> {
        // 1. Verify document exists.
        let _document = pipeline_repository::get_document(db, doc_id)
            .await
            .map_err(|e| CompletenessError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("get_document: {e}"),
            })?
            .ok_or_else(|| CompletenessError::DocumentNotFound {
                doc_id: doc_id.to_string(),
            })?;

        // 2. Get latest completed extraction run.
        let run_id = pipeline_repository::get_latest_completed_run(db, doc_id)
            .await
            .map_err(|e| CompletenessError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("get_latest_completed_run: {e}"),
            })?
            .ok_or_else(|| CompletenessError::NoCompletedRun {
                doc_id: doc_id.to_string(),
            })?;

        // 3. Fetch pipeline items and relationships.
        let items = pipeline_repository::get_approved_items_for_document(db, doc_id, run_id)
            .await
            .map_err(|e| CompletenessError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("get_approved_items: {e}"),
            })?;
        let rels = pipeline_repository::get_approved_relationships_for_document(db, run_id)
            .await
            .map_err(|e| CompletenessError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("get_approved_relationships: {e}"),
            })?;

        // 4. Group pipeline counts by type.
        let mut pipeline_items: HashMap<String, usize> = HashMap::new();
        for item in &items {
            *pipeline_items.entry(item.entity_type.clone()).or_insert(0) += 1;
        }
        let mut pipeline_rels: HashMap<String, usize> = HashMap::new();
        for rel in &rels {
            *pipeline_rels
                .entry(rel.relationship_type.clone())
                .or_insert(0) += 1;
        }

        // 5. Count Neo4j.
        let neo4j_nodes = count_neo4j_nodes_by_graph(&context.graph, doc_id)
            .await
            .map_err(|source| CompletenessError::Neo4j {
                doc_id: doc_id.to_string(),
                source,
            })?;
        let neo4j_rels = count_neo4j_relationships_by_graph(&context.graph, doc_id)
            .await
            .map_err(|source| CompletenessError::Neo4j {
                doc_id: doc_id.to_string(),
                source,
            })?;
        let orphaned = find_orphaned_nodes_by_graph(&context.graph, doc_id)
            .await
            .map_err(|source| CompletenessError::Neo4j {
                doc_id: doc_id.to_string(),
                source,
            })?;

        let neo4j_total_nodes: usize = neo4j_nodes.values().sum();
        let neo4j_total_rels: usize = neo4j_rels.values().sum();

        // 6. Count Qdrant. Filter on QDRANT_DOCUMENT_ID_FIELD — matches
        //    what the Index step wrote and what cleanup_qdrant uses.
        let qdrant_count = qdrant_service::count_points_by_filter(
            &context.http_client,
            &context.qdrant_url,
            QDRANT_DOCUMENT_ID_FIELD,
            doc_id,
        )
        .await
        .map_err(|e| CompletenessError::Qdrant {
            doc_id: doc_id.to_string(),
            message: e.to_string(),
        })?;

        // 7. Run comparison (pure function, no I/O).
        let checks = compare_counts(&CompareInput {
            pipeline_items: &pipeline_items,
            neo4j_nodes: &neo4j_nodes,
            pipeline_rels: &pipeline_rels,
            neo4j_rels: &neo4j_rels,
            total_pipeline_items: items.len(),
            total_pipeline_rels: rels.len(),
            neo4j_total_nodes,
            neo4j_total_rels,
            qdrant_count,
            orphaned_node_count: orphaned.len(),
        });

        let failed_checks: Vec<&CompletenessCheck> =
            checks.iter().filter(|c| c.status == "fail").collect();
        let warn_count = checks.iter().filter(|c| c.status == "warn").count();

        tracing::info!(
            doc_id = %doc_id,
            total_checks = checks.len(),
            failed = failed_checks.len(),
            warnings = warn_count,
            neo4j_total_nodes,
            neo4j_total_rels,
            qdrant_count,
            orphaned = orphaned.len(),
            "Completeness: checks evaluated"
        );

        // 8. Fail semantics: any "fail" → step error.
        if !failed_checks.is_empty() {
            let details = failed_checks
                .iter()
                .map(|c| format!("{}: {}", c.name, c.message))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(CompletenessError::ChecksFailed {
                doc_id: doc_id.to_string(),
                failed_count: failed_checks.len(),
                total_count: checks.len(),
                details,
            });
        }

        // 9. All checks passed → legacy status write.
        //
        // NOTE: writes STATUS_PUBLISHED (legacy) not DOC_STATUS_COMPLETED
        // (tracker). See module doc comment — Phase 5 PS-B8 owns the
        // lifecycle-migration coordination.
        pipeline_repository::update_document_status(db, doc_id, STATUS_PUBLISHED)
            .await
            .map_err(|e| CompletenessError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("update_document_status: {e}"),
            })?;

        Ok(CompletenessStats {
            checks_passed: checks.len() - failed_checks.len() - warn_count,
            checks_total: checks.len(),
            warnings: warn_count,
            neo4j_total_nodes,
            neo4j_total_rels,
            qdrant_count,
            orphaned_count: orphaned.len(),
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completeness_error_display_contains_doc_id() {
        let err = CompletenessError::DocumentNotFound {
            doc_id: "doc-xyz".to_string(),
        };
        assert!(format!("{err}").contains("doc-xyz"));
    }

    #[test]
    fn completeness_error_neo4j_display_excludes_source_text() {
        const UNIQUE: &str = "UNIQUE_COMPLETENESS_INNER_ERROR";
        let err = CompletenessError::Neo4j {
            doc_id: "doc-42".to_string(),
            source: neo4rs::Error::AuthenticationError(UNIQUE.to_string()),
        };
        let display = format!("{err}");
        assert!(display.contains("doc-42"));
        assert!(
            !display.contains(UNIQUE),
            "Display must not duplicate inner source (Kazlauskas G6); got: {display}"
        );
    }

    #[test]
    fn completeness_error_checks_failed_display_has_counts_and_details() {
        let err = CompletenessError::ChecksFailed {
            doc_id: "doc-7".to_string(),
            failed_count: 2,
            total_count: 5,
            details: "entity_foo: expected 10 got 8; qdrant_point_count: expected 15 got 12"
                .to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("doc-7"));
        assert!(display.contains("2 of 5"));
        assert!(display.contains("entity_foo"));
        assert!(display.contains("qdrant_point_count"));
    }

    #[test]
    fn completeness_step_constants_match_spec() {
        assert_eq!(Completeness::DEFAULT_RETRY_LIMIT, 3);
        assert_eq!(Completeness::DEFAULT_RETRY_DELAY_SECS, 10);
        assert_eq!(Completeness::DEFAULT_TIMEOUT_SECS, Some(60));
    }

    /// Lockstep guard. The legacy `STATUS_PUBLISHED` is what this step
    /// writes. If the PS-B8 lifecycle migration happens (rename to
    /// `COMPLETED`), this test must be updated in lockstep with the
    /// frontend and `state_machine` changes so nobody merges only half
    /// the rename.
    #[test]
    fn completeness_writes_published_not_completed() {
        assert_eq!(STATUS_PUBLISHED, "PUBLISHED");
    }
}
