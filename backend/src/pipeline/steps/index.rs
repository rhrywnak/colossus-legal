//! backend/src/pipeline/steps/index.rs
//!
//! Index step: generates embeddings for every Neo4j node belonging to
//! the document and upserts them to Qdrant. Reuses the existing
//! `embedding_repository`, `embedding_text`, and `qdrant_service`
//! helpers unchanged.
//!
//! ## Rust Learning: idempotency via Qdrant upsert
//!
//! Unlike P4-5's Ingest step (which uses cleanup-then-write because
//! `ingest_helpers` uses CREATE), Index is natively idempotent. Qdrant's
//! upsert semantics are "insert if absent, overwrite if present" —
//! re-running Index with the same node IDs produces the same points,
//! regardless of prior state. This is the canonical pattern per Qdrant
//! docs and per saga-step idempotency guidance (Temporal, AWS, Azure).
//!
//! Point IDs are derived from Neo4j node IDs via `DefaultHasher` —
//! deterministic within a Rust version. A Rust version upgrade that
//! changed the hasher would produce new point IDs and require a full
//! re-index, which is a full-cleanup-then-run operation anyway.
//!
//! ## Rust Learning: batch embedding via `EmbeddingProvider::embed_batch`
//!
//! The `EmbeddingProvider` trait has a default `embed_batch` that iterates
//! serially. `FastembedProvider` overrides it with a native batch call
//! using fastembed's rayon parallelism. For a ~300-node document this is
//! one `spawn_blocking` + one batch inference vs 300 `spawn_blocking`
//! calls. Always use `embed_batch` for multi-node index operations.
//!
//! ## Rust Learning: saga compensation via on_cancel
//!
//! `on_cancel` calls `cleanup_qdrant` to reverse partial upserts that
//! happened before the cancel signal. Since upsert is itself idempotent,
//! retry-after-cancel works correctly without special handling.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use colossus_pipeline::cancel::CancellationToken;
use colossus_pipeline::progress::ProgressReporter;
use colossus_pipeline::{Step, StepResult};

use crate::models::document_status::STATUS_INDEXED;
use crate::pipeline::constants::{QDRANT_COLLECTION_NAME, QDRANT_DOCUMENT_ID_FIELD};
use crate::pipeline::context::AppContext;
use crate::pipeline::steps::cleanup::{cleanup_qdrant, CleanupError};
use crate::pipeline::steps::completeness::Completeness;
use crate::pipeline::task::DocProcessing;
use crate::repositories::embedding_repository;
use crate::repositories::pipeline_repository;
use crate::services::embedding_text::build_embedding_text;
use crate::services::qdrant_service::{self, QdrantPoint};

/// Index step state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Index {
    pub document_id: String,
}

// ─────────────────────────────────────────────────────────────────────────
// IndexError
// ─────────────────────────────────────────────────────────────────────────

/// Failure modes for the Index step.
///
/// Per-subsystem variants carry the `doc_id` and thread the underlying
/// error via `#[source]` where the inner type is structurally useful.
/// Display strings exclude `{source}` so log output does not duplicate
/// inner messages (Kazlauskas G6).
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("No Neo4j nodes found for document '{doc_id}'")]
    NoNodes { doc_id: String },

    #[error("Embedding provider failed for document '{doc_id}'")]
    Embedding {
        doc_id: String,
        /// The message from `colossus_extract::PipelineError`. We don't
        /// carry `#[source]` here because `PipelineError`'s variants wrap
        /// `String` payloads — a source chain yields no more info than
        /// the `Display` text. Same pattern as P4-5's `Helper` variant.
        message: String,
    },

    #[error("Qdrant cleanup failed for document '{doc_id}'")]
    Cleanup {
        doc_id: String,
        #[source]
        source: CleanupError,
    },

    #[error("Helper failed for document '{doc_id}': {message}")]
    Helper { doc_id: String, message: String },
}

// ─────────────────────────────────────────────────────────────────────────
// Step impl
// ─────────────────────────────────────────────────────────────────────────

#[async_trait]
impl Step<DocProcessing> for Index {
    const DEFAULT_RETRY_LIMIT: i32 = 3;
    const DEFAULT_RETRY_DELAY_SECS: u64 = 10;
    const DEFAULT_TIMEOUT_SECS: Option<u64> = Some(300);

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
            return Err("Cancelled before indexing".into());
        }

        self.run_index(db, context, &doc_id).await?;

        if cancel.is_cancelled().await {
            return Err("Cancelled after indexing".into());
        }

        let duration_secs = start.elapsed().as_secs_f64();
        tracing::info!(
            doc_id = %doc_id,
            duration_secs,
            "Index step complete"
        );

        Ok(StepResult::Next(DocProcessing::Completeness(
            Completeness {
                document_id: self.document_id,
            },
        )))
    }

    async fn on_cancel(
        self,
        _db: &PgPool,
        context: &AppContext,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        cleanup_qdrant(&self.document_id, context)
            .await
            .map(|_| ())
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}

impl Index {
    /// Internal: perform the full embed-and-upsert path. Called from
    /// [`Step::execute`].
    async fn run_index(
        &self,
        db: &PgPool,
        context: &AppContext,
        doc_id: &str,
    ) -> Result<(), IndexError> {
        // 1. Ensure Qdrant collection exists (idempotent no-op if present).
        //    Dimension must match the provider's output — passed through
        //    per P2-Nx-A's parameterised signature.
        qdrant_service::ensure_collection(
            &context.http_client,
            &context.qdrant_url,
            context.embedding_provider.dimensions(),
        )
        .await
        .map_err(|e| IndexError::Helper {
            doc_id: doc_id.to_string(),
            message: format!("ensure_collection: {e}"),
        })?;

        // 2. Fetch nodes from Neo4j — the post-ingest source of truth.
        let nodes = embedding_repository::fetch_nodes_for_document(&context.graph, doc_id)
            .await
            .map_err(|e| IndexError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("fetch_nodes_for_document: {e}"),
            })?;

        if nodes.is_empty() {
            return Err(IndexError::NoNodes {
                doc_id: doc_id.to_string(),
            });
        }

        tracing::info!(
            doc_id = %doc_id,
            node_count = nodes.len(),
            "Index: fetched nodes from Neo4j"
        );

        // 3. Build embedding texts. Collect into `Vec<String>` to own the
        //    strings, then borrow for `embed_batch`.
        let texts: Vec<String> = nodes
            .iter()
            .map(|n| build_embedding_text(&n.node_type, &n.properties))
            .collect();
        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

        // 4. Batch embed via the provider abstraction. `FastembedProvider`
        //    overrides `embed_batch` to use fastembed's rayon-parallel
        //    batch API — single `spawn_blocking`, not N.
        let vectors = context
            .embedding_provider
            .embed_batch(&text_refs)
            .await
            .map_err(|e| IndexError::Embedding {
                doc_id: doc_id.to_string(),
                message: e.to_string(),
            })?;

        // Sanity check: provider must return one vector per input.
        if vectors.len() != nodes.len() {
            return Err(IndexError::Helper {
                doc_id: doc_id.to_string(),
                message: format!(
                    "embed_batch returned {} vectors for {} inputs",
                    vectors.len(),
                    nodes.len()
                ),
            });
        }

        // 5. Build Qdrant points. Payload schema matches the HTTP
        //    `index_handler` so downstream RAG queries work identically.
        let mut points: Vec<QdrantPoint> = Vec::with_capacity(nodes.len());
        let mut by_type: HashMap<String, usize> = HashMap::new();

        for (i, node) in nodes.iter().enumerate() {
            let vector = &vectors[i];
            *by_type.entry(node.node_type.clone()).or_insert(0) += 1;

            let title = node
                .properties
                .get("title")
                .or_else(|| node.properties.get("name"))
                .cloned()
                .unwrap_or_default();

            let mut payload = serde_json::json!({
                "node_id": node.id,
                "node_type": node.node_type,
                "title": title,
                QDRANT_DOCUMENT_ID_FIELD: doc_id,
                "source_document": doc_id,
            });

            // Attach page_number when present (Evidence nodes).
            if let Some(page) = node.properties.get("page_number") {
                if let Some(obj) = payload.as_object_mut() {
                    obj.insert(
                        "page_number".to_string(),
                        serde_json::Value::String(page.clone()),
                    );
                }
            }

            points.push(QdrantPoint {
                id: node_id_to_point_id(&node.id),
                vector: vector.clone(),
                payload,
            });
        }

        let embedded_count = points.len();

        // 6. Upsert to Qdrant (internal batching of 50/chunk).
        qdrant_service::upsert_points(&context.http_client, &context.qdrant_url, points)
            .await
            .map_err(|e| IndexError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("upsert_points: {e}"),
            })?;

        tracing::info!(
            doc_id = %doc_id,
            collection = QDRANT_COLLECTION_NAME,
            embedded_count,
            ?by_type,
            "Index: upserted points to Qdrant"
        );

        // 7. Legacy status write.
        //
        // NOTE: transitional. Frontend and state_machine.rs key off
        // documents.status = 'INDEXED'. The pipeline framework's own
        // pipeline_jobs.status is the canonical step-status source.
        // Phase 5 decides the fate of documents.status at the HTTP/UI
        // boundary.
        pipeline_repository::update_document_status(db, doc_id, STATUS_INDEXED)
            .await
            .map_err(|e| IndexError::Helper {
                doc_id: doc_id.to_string(),
                message: format!("update_document_status: {e}"),
            })?;

        Ok(())
    }
}

/// Convert a node ID string to a deterministic u64 for Qdrant point IDs.
///
/// Matches the private `api::pipeline::index::node_id_to_point_id` so
/// pipeline-path and HTTP-path writes produce compatible point IDs for
/// the same node. `DefaultHasher` is stable within a Rust version — a
/// version upgrade that changed the hasher would produce new IDs and
/// require a full re-index. The HTTP helper is module-private so we
/// replicate the implementation here rather than share a symbol.
fn node_id_to_point_id(node_id: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    node_id.hash(&mut hasher);
    hasher.finish()
}

// ─────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_error_display_contains_doc_id() {
        let err = IndexError::NoNodes {
            doc_id: "test-doc-42".to_string(),
        };
        assert!(format!("{err}").contains("test-doc-42"));
    }

    #[test]
    fn index_error_cleanup_variant_chains_source() {
        let inner = CleanupError::Neo4j {
            doc_id: "doc-7".to_string(),
            source: neo4rs::Error::AuthenticationError("inner".to_string()),
        };
        let err = IndexError::Cleanup {
            doc_id: "doc-7".to_string(),
            source: inner,
        };
        use std::error::Error as _;
        assert!(err.source().is_some(), "source() must return Some");
    }

    #[test]
    fn node_id_to_point_id_is_deterministic() {
        let a = node_id_to_point_id("complaint-allegation-1");
        let b = node_id_to_point_id("complaint-allegation-1");
        assert_eq!(a, b, "hashing must be deterministic within a run");
    }

    #[test]
    fn node_id_to_point_id_differs_for_different_ids() {
        let a = node_id_to_point_id("complaint-allegation-1");
        let b = node_id_to_point_id("complaint-allegation-2");
        assert_ne!(a, b);
    }

    #[test]
    fn index_step_constants_match_spec() {
        assert_eq!(Index::DEFAULT_RETRY_LIMIT, 3);
        assert_eq!(Index::DEFAULT_RETRY_DELAY_SECS, 10);
        assert_eq!(Index::DEFAULT_TIMEOUT_SECS, Some(300));
    }

    /// Compile-time guard: `QDRANT_DOCUMENT_ID_FIELD` must equal
    /// `"document_id"` for the payload JSON key to align with
    /// `cleanup_qdrant`'s filter key and with the HTTP `index_handler`'s
    /// payload shape.
    #[test]
    fn qdrant_document_id_field_matches_expected_value() {
        assert_eq!(QDRANT_DOCUMENT_ID_FIELD, "document_id");
    }
}
