//! Restate workflow step: write approved extraction items into Neo4j.
//!
//! Wraps the clean
//! [`run_ingest`](crate::pipeline::steps::ingest::run_ingest)
//! orchestrator with the Restate error classification. No
//! `documents.status` write here — the core function writes
//! `STATUS_INGESTED` itself (same pattern as completeness).
//!
//! ## Idempotency on the Restate path
//!
//! `run_ingest` performs cleanup-then-write: it calls
//! `cleanup_neo4j` first to wipe any partial Neo4j state from a
//! prior attempt, then writes fresh. Restate replay
//! (which re-executes the `ctx.run` closure body on workflow
//! recovery) gets cleanup-then-write idempotency for free —
//! no Restate-layer cleanup call needed.
//!
//! The underlying `ingest_helpers` uses `CREATE` (not `MERGE`) for
//! everything except Party entities; cleanup-then-write is the
//! bounded-cost workaround until the cross-cutting
//! **P-MERGE-refactor** lands.

use std::sync::Arc;

use restate_sdk::errors::{HandlerError, TerminalError};

use crate::pipeline::context::AppContext;
use crate::pipeline::steps::ingest::{run_ingest, IngestError};

/// Restate workflow step: ingest approved extraction items into Neo4j.
#[tracing::instrument(skip(app), fields(doc_id = %doc_id, step = "ingest"))]
pub async fn step_ingest(app: &Arc<AppContext>, doc_id: &str) -> Result<String, HandlerError> {
    let result = run_ingest(doc_id, &app.pipeline_pool, app.as_ref())
        .await
        .map_err(|e| classify_ingest_error(doc_id, &e))?;

    let summary = format!(
        "ingest_complete nodes={} relationships={}",
        result.total_nodes, result.total_rels
    );
    tracing::info!(
        doc_id = %doc_id,
        total_nodes = result.total_nodes,
        total_rels = result.total_rels,
        "step_ingest: complete"
    );
    Ok(summary)
}

/// Classify an [`IngestError`] as terminal or retryable.
///
/// Data-state issues (missing document, no completed pass-1 run)
/// are terminal — the retry will see the same state. Transient
/// infrastructure failures (Neo4j connection blips, cleanup
/// failures, helper-layer DB timeouts) are retryable.
fn classify_ingest_error(doc_id: &str, e: &IngestError) -> HandlerError {
    use IngestError as E;
    match e {
        E::DocumentNotFound { .. } => TerminalError::new(format!(
            "step_ingest: document '{doc_id}' not found in database. \
             Confirm the upload completed before invoking the workflow."
        ))
        .into(),
        E::NoCompletedRun { .. } => TerminalError::new(format!(
            "step_ingest: no COMPLETED extraction_run for '{doc_id}'. \
             Pass-1 (and pass-2 if configured) must succeed before \
             ingest can write entities to Neo4j."
        ))
        .into(),
        E::Cleanup { .. } => HandlerError::from(format!(
            "step_ingest: transient pre-run Neo4j cleanup failure for \
             '{doc_id}': {e}. Will retry."
        )),
        E::Neo4j { .. } => HandlerError::from(format!(
            "step_ingest: transient Neo4j failure for '{doc_id}': {e}. \
             Will retry."
        )),
        E::Helper { .. } => HandlerError::from(format!(
            "step_ingest: transient helper failure for '{doc_id}': {e}. \
             Will retry."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display_message(e: &HandlerError) -> String {
        let inner: &dyn std::error::Error = e.as_ref();
        format!("{inner}")
    }

    fn is_terminal(e: &HandlerError) -> bool {
        display_message(e).starts_with("Terminal error")
    }

    #[test]
    fn classify_document_not_found_is_terminal() {
        let err = IngestError::DocumentNotFound {
            doc_id: "doc-x".into(),
        };
        let c = classify_ingest_error("doc-x", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(msg.contains("doc-x"));
        assert!(msg.contains("upload completed"));
    }

    #[test]
    fn classify_no_completed_run_is_terminal() {
        let err = IngestError::NoCompletedRun {
            doc_id: "doc-x".into(),
        };
        let c = classify_ingest_error("doc-x", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(msg.contains("Pass-1"));
    }

    #[test]
    fn classify_neo4j_is_retryable() {
        let err = IngestError::Neo4j {
            doc_id: "doc-x".into(),
            source: neo4rs::Error::AuthenticationError("connection refused".into()),
        };
        let c = classify_ingest_error("doc-x", &err);
        assert!(!is_terminal(&c));
        let msg = display_message(&c);
        assert!(msg.contains("Will retry"));
    }

    #[test]
    fn classify_helper_is_retryable() {
        let err = IngestError::Helper {
            doc_id: "doc-x".into(),
            message: "neo4j_node_id lookup failed".into(),
        };
        let c = classify_ingest_error("doc-x", &err);
        assert!(!is_terminal(&c));
    }

    #[test]
    fn classify_cleanup_is_retryable() {
        // The Cleanup arm has a distinct message prefix ("pre-run
        // Neo4j cleanup") that the Helper / Neo4j tests above do not
        // exercise. Synthesize a `CleanupError::Neo4j` via the same
        // `neo4rs::Error::AuthenticationError` constructor the
        // pre-existing Display tests in `steps/ingest.rs` already use.
        let inner = crate::pipeline::steps::cleanup::CleanupError::Neo4j {
            doc_id: "doc-x".into(),
            source: neo4rs::Error::AuthenticationError("simulated".into()),
        };
        let err = IngestError::Cleanup {
            doc_id: "doc-x".into(),
            source: inner,
        };
        let c = classify_ingest_error("doc-x", &err);
        assert!(!is_terminal(&c));
        let msg = display_message(&c);
        assert!(
            msg.contains("pre-run Neo4j cleanup"),
            "msg must surface the cleanup-specific prefix: {msg}"
        );
        assert!(msg.contains("Will retry"));
    }
}
