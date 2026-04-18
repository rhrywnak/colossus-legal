//! backend/src/pipeline/steps/cleanup.rs
//!
//! Teardown helpers for removing a document's derived state from the three
//! external stores colossus-legal writes to: Neo4j, Qdrant, and PostgreSQL.
//!
//! Why it exists in this system: document deletion, ingestion rollback, and
//! step `on_cancel` / `on_delete` hooks all need the same typed, idempotent
//! teardown primitives. Centralising them here keeps the delete-path logic
//! out of individual step implementations and replaces the existing
//! "best-effort log and swallow" pattern in `api/pipeline/delete.rs` with a
//! composable, error-propagating API.
//!
//! ## Rust Learning: Saga pattern with partial-failure reporting
//!
//! `cleanup_all` coordinates three independent teardown operations. Each
//! can fail in isolation. Rather than short-circuiting on the first error,
//! we run all three, collect per-subsystem successes into a
//! `CleanupReport`, and return `CleanupError::Partial` when any subsystem
//! failed — so the caller sees exactly which stores were cleared and which
//! still need manual intervention.
//!
//! ## Rust Learning: `#[source]` threading without leaking inner Display
//!
//! `thiserror`'s `#[source]` attribute threads the wrapped error through
//! `std::error::Error::source()`. `tracing` and `eyre`-style printers walk
//! that chain automatically. We deliberately omit `{source}` from the
//! top-level `#[error(...)]` Display strings so log output does not
//! duplicate the inner message (Kazlauskas Guideline 6).

use neo4rs::Graph;
use serde::Serialize;
use sqlx::PgPool;

use crate::pipeline::constants::{
    NEO4J_SOURCE_DOCUMENT_ID_PROP, NEO4J_SOURCE_DOCUMENT_PROP, QDRANT_COLLECTION_NAME,
    QDRANT_DOCUMENT_ID_FIELD,
};
use crate::pipeline::context::AppContext;
use crate::services::qdrant_service::{self, QdrantError};

// ─────────────────────────────────────────────────────────────────────────
// Error type
// ─────────────────────────────────────────────────────────────────────────

/// Failure modes for document teardown.
///
/// Per-subsystem variants carry the `doc_id` being cleaned and the
/// underlying error as a `#[source]` chain. The `Partial` variant wraps
/// sub-errors so a single failed subsystem does not mask successful
/// teardown on the other two.
#[derive(Debug, thiserror::Error)]
pub enum CleanupError {
    #[error("Neo4j cleanup failed for document {doc_id}")]
    Neo4j {
        doc_id: String,
        #[source]
        source: neo4rs::Error,
    },

    #[error("Qdrant cleanup failed for document {doc_id}")]
    Qdrant {
        doc_id: String,
        #[source]
        source: QdrantError,
    },

    #[error("Postgres cleanup failed for document {doc_id}")]
    Postgres {
        doc_id: String,
        #[source]
        source: sqlx::Error,
    },

    #[error("Partial cleanup failure for document {doc_id}")]
    Partial {
        doc_id: String,
        neo4j_error: Option<Box<CleanupError>>,
        qdrant_error: Option<Box<CleanupError>>,
        postgres_error: Option<Box<CleanupError>>,
        partial_report: CleanupReport,
    },
}

// ─────────────────────────────────────────────────────────────────────────
// Report types
// ─────────────────────────────────────────────────────────────────────────

/// Counts of Neo4j nodes removed, split by which `source_document*`
/// property matched.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Neo4jCleanupReport {
    pub nodes_by_source_document: i64,
    pub nodes_by_source_document_id: i64,
}

/// Count of Qdrant vectors removed from the evidence collection.
#[derive(Debug, Default, Clone, Serialize)]
pub struct QdrantCleanupReport {
    pub vectors_deleted: u64,
}

/// Per-table row counts removed from PostgreSQL, in the order the DELETEs
/// were issued (FK-safe).
#[derive(Debug, Default, Clone, Serialize)]
pub struct PostgresCleanupReport {
    pub tables_cleared: Vec<(&'static str, u64)>,
}

/// Composite report returned by [`cleanup_all`] on full success, and
/// attached to [`CleanupError::Partial`] when some subsystems succeeded.
#[derive(Debug, Default, Clone, Serialize)]
pub struct CleanupReport {
    pub neo4j: Neo4jCleanupReport,
    pub qdrant: QdrantCleanupReport,
    pub postgres: PostgresCleanupReport,
}

// ─────────────────────────────────────────────────────────────────────────
// cleanup_neo4j
// ─────────────────────────────────────────────────────────────────────────

/// Remove every Neo4j node whose `source_document` or `source_document_id`
/// property matches the given document id. Runs two DETACH DELETE queries
/// and reports per-property counts.
///
/// Idempotent: re-running on a doc_id with no matching nodes returns a
/// zero-valued report.
pub async fn cleanup_neo4j(
    document_id: &str,
    graph: &Graph,
) -> Result<Neo4jCleanupReport, CleanupError> {
    let by_source_document =
        delete_nodes_by_property(graph, document_id, NEO4J_SOURCE_DOCUMENT_PROP).await?;
    let by_source_document_id =
        delete_nodes_by_property(graph, document_id, NEO4J_SOURCE_DOCUMENT_ID_PROP).await?;
    Ok(Neo4jCleanupReport {
        nodes_by_source_document: by_source_document,
        nodes_by_source_document_id: by_source_document_id,
    })
}

/// Execute a single property-scoped DETACH DELETE and return the reported
/// `count(n)` for logging. The Cypher template lives here because
/// [`crate::pipeline::constants`] supplies the property names; inlining
/// into `cleanup_neo4j` would duplicate the error-mapping boilerplate.
async fn delete_nodes_by_property(
    graph: &Graph,
    document_id: &str,
    property: &str,
) -> Result<i64, CleanupError> {
    let cypher = format!(
        "MATCH (n) WHERE n.{property} = $doc_id DETACH DELETE n RETURN count(n) AS removed"
    );
    let mut result = graph
        .execute(neo4rs::query(&cypher).param("doc_id", document_id))
        .await
        .map_err(|source| CleanupError::Neo4j {
            doc_id: document_id.to_string(),
            source,
        })?;
    let removed = match result.next().await {
        Ok(Some(row)) => row.get::<i64>("removed").unwrap_or(0),
        Ok(None) => 0,
        Err(source) => {
            return Err(CleanupError::Neo4j {
                doc_id: document_id.to_string(),
                source,
            })
        }
    };
    Ok(removed)
}

// ─────────────────────────────────────────────────────────────────────────
// cleanup_qdrant
// ─────────────────────────────────────────────────────────────────────────

/// Remove every Qdrant vector whose `document_id` payload matches.
///
/// Uses [`qdrant_service::delete_points_by_filter`] with the
/// [`QDRANT_DOCUMENT_ID_FIELD`] constant. The collection name is fixed
/// inside `qdrant_service` — [`QDRANT_COLLECTION_NAME`] is referenced here
/// solely for observability.
pub async fn cleanup_qdrant(
    document_id: &str,
    context: &AppContext,
) -> Result<QdrantCleanupReport, CleanupError> {
    let count = qdrant_service::delete_points_by_filter(
        &context.http_client,
        &context.qdrant_url,
        QDRANT_DOCUMENT_ID_FIELD,
        document_id,
    )
    .await
    .map_err(|source| CleanupError::Qdrant {
        doc_id: document_id.to_string(),
        source,
    })?;
    tracing::debug!(
        doc_id = %document_id,
        collection = QDRANT_COLLECTION_NAME,
        count,
        "cleanup_qdrant removed vectors"
    );
    Ok(QdrantCleanupReport {
        vectors_deleted: count as u64,
    })
}

// ─────────────────────────────────────────────────────────────────────────
// cleanup_postgres
// ─────────────────────────────────────────────────────────────────────────

/// PostgreSQL delete order. Listed FK-safe: children before parents. The
/// `extraction_chunks` subquery resolves through `extraction_runs`, which
/// the next entry then clears.
///
/// `pipeline_steps` and `document_audit_log` are intentionally omitted —
/// the former is framework-owned (colossus-pipeline), the latter must
/// survive deletion by design.
#[rustfmt::skip]
const POSTGRES_DELETE_ORDER: &[(&str, &str)] = &[
    ("extraction_relationships", "DELETE FROM extraction_relationships WHERE document_id = $1"),
    ("extraction_items",          "DELETE FROM extraction_items WHERE document_id = $1"),
    ("extraction_chunks",         "DELETE FROM extraction_chunks WHERE extraction_run_id IN (SELECT id FROM extraction_runs WHERE document_id = $1)"),
    ("extraction_runs",           "DELETE FROM extraction_runs WHERE document_id = $1"),
    ("document_text",             "DELETE FROM document_text WHERE document_id = $1"),
    ("pipeline_config",           "DELETE FROM pipeline_config WHERE document_id = $1"),
];

/// Delete all PostgreSQL rows keyed by `document_id` in one transaction.
///
/// The transaction auto-rolls-back on drop, so returning
/// [`CleanupError::Postgres`] from mid-loop leaves the database untouched.
/// On the happy path, commit happens after the final DELETE succeeds.
pub async fn cleanup_postgres(
    document_id: &str,
    db: &PgPool,
) -> Result<PostgresCleanupReport, CleanupError> {
    let mut txn = db
        .begin()
        .await
        .map_err(|source| postgres_err(document_id, source))?;
    let mut tables_cleared: Vec<(&'static str, u64)> =
        Vec::with_capacity(POSTGRES_DELETE_ORDER.len());

    for (table, sql) in POSTGRES_DELETE_ORDER {
        let result = sqlx::query(sql)
            .bind(document_id)
            .execute(&mut *txn)
            .await
            .map_err(|source| postgres_err(document_id, source))?;
        tables_cleared.push((*table, result.rows_affected()));
    }
    txn.commit()
        .await
        .map_err(|source| postgres_err(document_id, source))?;
    Ok(PostgresCleanupReport { tables_cleared })
}

fn postgres_err(doc_id: &str, source: sqlx::Error) -> CleanupError {
    CleanupError::Postgres {
        doc_id: doc_id.to_string(),
        source,
    }
}

// ─────────────────────────────────────────────────────────────────────────
// cleanup_all — saga pattern
// ─────────────────────────────────────────────────────────────────────────

/// Run all three teardown helpers, collecting per-subsystem errors without
/// short-circuiting. Returns `Ok(report)` iff every subsystem succeeded;
/// otherwise returns [`CleanupError::Partial`] with the composite report
/// showing which stores were cleared.
pub async fn cleanup_all(
    document_id: &str,
    db: &PgPool,
    context: &AppContext,
) -> Result<CleanupReport, CleanupError> {
    let mut report = CleanupReport::default();

    let neo4j_error = match cleanup_neo4j(document_id, &context.graph).await {
        Ok(r) => {
            report.neo4j = r;
            None
        }
        Err(e) => Some(Box::new(e)),
    };
    let qdrant_error = match cleanup_qdrant(document_id, context).await {
        Ok(r) => {
            report.qdrant = r;
            None
        }
        Err(e) => Some(Box::new(e)),
    };
    let postgres_error = match cleanup_postgres(document_id, db).await {
        Ok(r) => {
            report.postgres = r;
            None
        }
        Err(e) => Some(Box::new(e)),
    };

    if neo4j_error.is_none() && qdrant_error.is_none() && postgres_error.is_none() {
        tracing::info!(doc_id = %document_id, ?report, "cleanup_all succeeded");
        Ok(report)
    } else {
        tracing::error!(
            doc_id = %document_id,
            ?neo4j_error,
            ?qdrant_error,
            ?postgres_error,
            ?report,
            "cleanup_all partial failure"
        );
        Err(CleanupError::Partial {
            doc_id: document_id.to_string(),
            neo4j_error,
            qdrant_error,
            postgres_error,
            partial_report: report,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Distinctive payload used to prove that the outer Display of
    /// [`CleanupError::Neo4j`] does NOT duplicate the inner source text.
    const UNIQUE_INNER: &str = "UNIQUE_INNER_ERROR_MESSAGE";

    /// Build a [`neo4rs::Error`] whose Display equals [`UNIQUE_INNER`] so
    /// the "source not in outer" assertion in
    /// `cleanup_error_neo4j_display_includes_doc_id_not_source` is exact.
    /// `AuthenticationError(String)` formats with `"{0}"`, giving the raw
    /// payload string verbatim in the inner Display.
    fn dummy_neo4j_err() -> neo4rs::Error {
        neo4rs::Error::AuthenticationError(UNIQUE_INNER.to_string())
    }

    #[test]
    fn cleanup_report_default_is_all_zero() {
        let r = CleanupReport::default();
        assert_eq!(r.neo4j.nodes_by_source_document, 0);
        assert_eq!(r.neo4j.nodes_by_source_document_id, 0);
        assert_eq!(r.qdrant.vectors_deleted, 0);
        assert!(r.postgres.tables_cleared.is_empty());
    }

    #[test]
    fn cleanup_error_neo4j_display_includes_doc_id_not_source() {
        let err = CleanupError::Neo4j {
            doc_id: "doc-42".to_string(),
            source: dummy_neo4j_err(),
        };
        let display = format!("{err}");

        // Sanity check: the inner error really does carry UNIQUE_INNER.
        let inner_display = format!("{}", dummy_neo4j_err());
        assert_eq!(
            inner_display, UNIQUE_INNER,
            "dummy inner Display should equal the sentinel; got {inner_display}"
        );

        assert!(
            display.contains("doc-42"),
            "outer Display must include doc_id, got: {display}"
        );
        assert!(
            !display.contains(UNIQUE_INNER),
            "outer Display must NOT duplicate inner source text (Kazlauskas 6), got: {display}"
        );
    }

    #[test]
    fn cleanup_error_partial_display_names_subsystems() {
        let inner = CleanupError::Neo4j {
            doc_id: "doc-7".to_string(),
            source: dummy_neo4j_err(),
        };
        let err = CleanupError::Partial {
            doc_id: "doc-7".to_string(),
            neo4j_error: Some(Box::new(inner)),
            qdrant_error: None,
            postgres_error: None,
            partial_report: CleanupReport::default(),
        };
        let display = format!("{err}");
        assert!(
            display.contains("doc-7"),
            "Partial Display must include doc_id, got: {display}"
        );
    }

    #[test]
    fn cleanup_report_serializes_to_json() {
        let report = CleanupReport {
            neo4j: Neo4jCleanupReport {
                nodes_by_source_document: 5,
                nodes_by_source_document_id: 1,
            },
            qdrant: QdrantCleanupReport {
                vectors_deleted: 12,
            },
            postgres: PostgresCleanupReport {
                tables_cleared: vec![("extraction_items", 7)],
            },
        };
        let json = serde_json::to_value(&report).expect("serialize CleanupReport");
        let obj = json.as_object().expect("top-level JSON object");
        assert!(obj.contains_key("neo4j"), "missing neo4j key: {json}");
        assert!(obj.contains_key("qdrant"), "missing qdrant key: {json}");
        assert!(obj.contains_key("postgres"), "missing postgres key: {json}");
        assert_eq!(json["neo4j"]["nodes_by_source_document"], 5);
        assert_eq!(json["neo4j"]["nodes_by_source_document_id"], 1);
        assert_eq!(json["qdrant"]["vectors_deleted"], 12);
        assert_eq!(json["postgres"]["tables_cleared"][0][0], "extraction_items");
        assert_eq!(json["postgres"]["tables_cleared"][0][1], 7);
    }
}
