//! Restate workflow step: embed Neo4j nodes and upsert into Qdrant.
//!
//! Wraps the clean [`run_index`](crate::pipeline::steps::index::run_index)
//! orchestrator with the Restate error classification. No
//! `documents.status` write here — the core function writes
//! `STATUS_INDEXED` itself.
//!
//! ## Idempotency
//!
//! Qdrant upsert is natively idempotent. Point IDs are deterministic
//! (derived from Neo4j node IDs via `DefaultHasher`). Restate replay
//! re-executes the embed+upsert path and produces identical points
//! — no Restate-layer cleanup or guard needed.

use std::sync::Arc;

use restate_sdk::errors::{HandlerError, TerminalError};

use crate::pipeline::context::AppContext;
use crate::pipeline::steps::index::{run_index, IndexError};

/// Restate workflow step: embed Neo4j nodes and upsert into Qdrant.
#[tracing::instrument(skip(app), fields(doc_id = %doc_id, step = "index"))]
pub async fn step_index(app: &Arc<AppContext>, doc_id: &str) -> Result<String, HandlerError> {
    let result = run_index(doc_id, &app.pipeline_pool, app.as_ref())
        .await
        .map_err(|e| classify_index_error(doc_id, &e))?;

    let summary = format!("index_complete points_indexed={}", result.embedded_count);
    tracing::info!(
        doc_id = %doc_id,
        embedded_count = result.embedded_count,
        "step_index: complete"
    );
    Ok(summary)
}

/// Classify an [`IndexError`] as terminal or retryable.
///
/// Only `NoNodes` is terminal (Ingest didn't produce nodes; retry
/// won't fix that). Embedding / Qdrant / helper failures are
/// retryable (rate limits, network blips).
fn classify_index_error(doc_id: &str, e: &IndexError) -> HandlerError {
    use IndexError as E;
    match e {
        E::NoNodes { .. } => TerminalError::new(format!(
            "step_index: no Neo4j nodes for '{doc_id}'. Ingest must have \
             produced nodes before Index can embed them — investigate the \
             ingest step's log output."
        ))
        .into(),
        E::Embedding { .. } => HandlerError::from(format!(
            "step_index: transient embedding-provider failure for '{doc_id}': \
             {e}. Will retry."
        )),
        E::Cleanup { .. } => HandlerError::from(format!(
            "step_index: transient Qdrant cleanup failure for '{doc_id}': \
             {e}. Will retry."
        )),
        E::Helper { .. } => HandlerError::from(format!(
            "step_index: transient helper failure for '{doc_id}': {e}. \
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
    fn classify_no_nodes_is_terminal() {
        let err = IndexError::NoNodes {
            doc_id: "doc-x".into(),
        };
        let c = classify_index_error("doc-x", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(msg.contains("doc-x"));
        assert!(msg.contains("Ingest must have produced"));
    }

    #[test]
    fn classify_embedding_is_retryable() {
        let err = IndexError::Embedding {
            doc_id: "doc-x".into(),
            message: "rate limited".into(),
        };
        let c = classify_index_error("doc-x", &err);
        assert!(!is_terminal(&c));
        let msg = display_message(&c);
        assert!(msg.contains("Will retry"));
    }

    #[test]
    fn classify_helper_is_retryable() {
        let err = IndexError::Helper {
            doc_id: "doc-x".into(),
            message: "qdrant upsert returned 500".into(),
        };
        let c = classify_index_error("doc-x", &err);
        assert!(!is_terminal(&c));
    }

    #[test]
    fn classify_cleanup_is_retryable() {
        // The Cleanup arm has a distinct "Qdrant cleanup" message
        // prefix the Embedding / Helper tests above don't exercise.
        // Synthesize a CleanupError via the Neo4j variant — it's
        // the only `CleanupError` shape we can build without
        // touching `QdrantError`'s internals, and the classify
        // function doesn't pattern-match on the inner variant.
        let inner = crate::pipeline::steps::cleanup::CleanupError::Neo4j {
            doc_id: "doc-x".into(),
            source: neo4rs::Error::AuthenticationError("simulated".into()),
        };
        let err = IndexError::Cleanup {
            doc_id: "doc-x".into(),
            source: inner,
        };
        let c = classify_index_error("doc-x", &err);
        assert!(!is_terminal(&c));
        let msg = display_message(&c);
        assert!(
            msg.contains("Qdrant cleanup"),
            "msg must surface the cleanup-specific prefix: {msg}"
        );
        assert!(msg.contains("Will retry"));
    }
}
