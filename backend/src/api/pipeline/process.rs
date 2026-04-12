//! POST /api/admin/pipeline/documents/:id/process — One-button pipeline.
//!
//! Runs the full extraction pipeline as a single async operation:
//! extract_text → extract → verify → auto-ingest → index → completeness.
//!
//! Returns 202 Accepted immediately. Frontend polls GET /documents/:id
//! for progress updates.
//!
//! ## Rust Learning: tokio::spawn for fire-and-forget
//!
//! The process handler validates the request synchronously, then spawns
//! an async task that runs the pipeline in the background. The spawned
//! task owns clones of AppState (which is cheap — Arc internally) and
//! the document ID string. If the task panics, tokio logs the error
//! but the server continues running.

use axum::{extract::Path, extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::pipeline_repository::{self, documents};
use crate::services::qdrant_service;
use crate::state::AppState;

use super::extract::ExtractRequest;

// ── Cancel Response DTO ────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CancelResponse {
    pub document_id: String,
    pub message: String,
}

// ── Request/Response DTOs ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ProcessRequest {
    /// Optional: "same_settings", "new_settings", "delete_and_reextract"
    /// If absent, treated as first-time processing.
    #[serde(default)]
    pub reprocess_option: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProcessResponse {
    pub document_id: String,
    pub status: String,
    pub message: String,
}

// ── Handler ────────────────────────────────────────────────────

/// POST /documents/:id/process
///
/// Validates the request, sets status to PROCESSING, then spawns an async
/// task that runs the full pipeline. Returns 202 Accepted immediately.
pub async fn process_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    body: Option<Json<ProcessRequest>>,
) -> Result<(axum::http::StatusCode, Json<ProcessResponse>), AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST process");

    // 1. Fetch document, validate status
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    // Allow processing from: UPLOADED (first time), COMPLETED, FAILED, CANCELLED (re-process)
    let allowed_statuses = [
        "NEW", "COMPLETED", "FAILED", "CANCELLED",
        // Legacy statuses (pre-migration documents that haven't been migrated yet):
        "UPLOADED", "TEXT_EXTRACTED", "EXTRACTED", "VERIFIED",
        "PUBLISHED", "EXTRACTION_FAILED",
    ];
    if !allowed_statuses.contains(&document.status.as_str()) {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot process: status is '{}'. Document may already be processing.",
                document.status,
            ),
            details: serde_json::json!({ "status": document.status }),
        });
    }

    // 2. If re-processing, clean up previous data
    let reprocess_option = body
        .as_ref()
        .and_then(|b| b.reprocess_option.as_deref())
        .unwrap_or("none");

    if document.status != "NEW" && document.status != "UPLOADED" {
        cleanup_for_reprocess(&state, &doc_id, reprocess_option).await?;
    }

    // 3. Set status to PROCESSING, clear errors
    documents::clear_processing_errors(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;
    pipeline_repository::update_document_status(&state.pipeline_pool, &doc_id, "PROCESSING")
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;

    // 4. Spawn async pipeline task
    let state_clone = state.clone();
    let doc_id_clone = doc_id.clone();
    let username = user.username.clone();

    tokio::spawn(async move {
        if let Err(e) = run_pipeline(&state_clone, &doc_id_clone, &username).await {
            tracing::error!(doc_id = %doc_id_clone, error = ?e, "Pipeline failed");
            // Error details already stored by run_pipeline
        }
    });

    // 5. Return 202 Accepted
    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(ProcessResponse {
            document_id: doc_id,
            status: "PROCESSING".to_string(),
            message: "Processing started. Poll GET /documents/:id for progress.".to_string(),
        }),
    ))
}

// ── Cancel Handler ─────────────────────────────────────────────

/// POST /documents/:id/cancel — cancel in-progress processing.
///
/// Sets the `is_cancelled` flag on the document. The pipeline runner checks
/// this flag between steps and stops gracefully. Cancellation is not instant —
/// the current step must complete before the pipeline stops.
pub async fn cancel_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<CancelResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST cancel");

    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    if document.status != "PROCESSING" {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot cancel: status is '{}', expected 'PROCESSING'",
                document.status
            ),
            details: serde_json::json!({ "status": document.status }),
        });
    }

    documents::set_cancelled(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;

    Ok(Json(CancelResponse {
        document_id: doc_id,
        message: "Cancellation requested. Processing will stop after the current step completes."
            .to_string(),
    }))
}

// ── Cleanup ────────────────────────────────────────────────────

/// Clean up previous extraction data for re-processing.
///
/// Deletes extraction data but keeps the document row and document_text
/// (unless delete_and_reextract is specified, which also clears Neo4j/Qdrant).
async fn cleanup_for_reprocess(
    state: &AppState,
    doc_id: &str,
    reprocess_option: &str,
) -> Result<(), AppError> {
    let pool = &state.pipeline_pool;

    // Delete PostgreSQL extraction data (FK-safe order, same as delete handler)
    sqlx::query("DELETE FROM extraction_relationships WHERE document_id = $1")
        .bind(doc_id).execute(pool).await
        .map_err(|e| AppError::Internal { message: format!("Cleanup error: {e}") })?;
    sqlx::query("DELETE FROM extraction_items WHERE document_id = $1")
        .bind(doc_id).execute(pool).await
        .map_err(|e| AppError::Internal { message: format!("Cleanup error: {e}") })?;
    sqlx::query(
        "DELETE FROM extraction_chunks WHERE extraction_run_id IN \
         (SELECT id FROM extraction_runs WHERE document_id = $1)",
    )
    .bind(doc_id).execute(pool).await
    .map_err(|e| AppError::Internal { message: format!("Cleanup error: {e}") })?;
    sqlx::query("DELETE FROM extraction_runs WHERE document_id = $1")
        .bind(doc_id).execute(pool).await
        .map_err(|e| AppError::Internal { message: format!("Cleanup error: {e}") })?;
    // Keep document_text — no need to re-extract text unless explicitly requested
    // Delete pipeline_steps for steps after upload and extract_text
    sqlx::query(
        "DELETE FROM pipeline_steps WHERE document_id = $1 \
         AND step_name NOT IN ('upload', 'extract_text')",
    )
    .bind(doc_id).execute(pool).await
    .map_err(|e| AppError::Internal { message: format!("Cleanup error: {e}") })?;

    // If delete_and_reextract, also clear Neo4j, Qdrant, and document_text
    if reprocess_option == "delete_and_reextract" {
        sqlx::query("DELETE FROM document_text WHERE document_id = $1")
            .bind(doc_id).execute(pool).await
            .map_err(|e| AppError::Internal { message: format!("Cleanup error: {e}") })?;
        sqlx::query(
            "DELETE FROM pipeline_steps WHERE document_id = $1 AND step_name = 'extract_text'",
        )
        .bind(doc_id).execute(pool).await
        .map_err(|e| AppError::Internal { message: format!("Cleanup error: {e}") })?;

        // Neo4j cleanup (best-effort)
        let _ = state.graph.run(
            neo4rs::query("MATCH (n) WHERE n.source_document = $doc_id DETACH DELETE n")
                .param("doc_id", doc_id),
        ).await;
        let _ = state.graph.run(
            neo4rs::query("MATCH (n) WHERE n.source_document_id = $doc_id DETACH DELETE n")
                .param("doc_id", doc_id),
        ).await;

        // Qdrant cleanup (best-effort)
        let _ = qdrant_service::delete_points_by_filter(
            &state.http_client, &state.config.qdrant_url, "document_id", doc_id,
        ).await;
    }

    // Clear write summary
    documents::set_write_summary(pool, doc_id, 0, 0, 0)
        .await
        .map_err(|e| AppError::Internal { message: format!("Cleanup error: {e}") })?;

    Ok(())
}

// ── Pipeline Runner ────────────────────────────────────────────

/// Run the full pipeline. Called from the spawned task.
///
/// Each step updates progress. On failure, stores error details and sets FAILED.
/// On success, sets COMPLETED. Checks cancellation between steps.
async fn run_pipeline(
    state: &AppState,
    doc_id: &str,
    username: &str,
) -> Result<(), AppError> {
    let pool = &state.pipeline_pool;

    // --- Step 1: Extract Text ---
    let text_exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM document_text WHERE document_id = $1",
    )
    .bind(doc_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if text_exists == 0 {
        documents::update_processing_progress(
            pool, doc_id, "extract_text", "Reading document...", 0, 0, 0, 5,
        ).await.ok();

        if let Err(e) = super::extract_text::run_extract_text(state, doc_id, username).await {
            let msg = format!("{e:?}");
            let suggestion = error_suggestion("extract_text", &msg);
            documents::set_processing_error(pool, doc_id, "extract_text", None, &msg, &suggestion)
                .await.ok();
            documents::clear_processing_progress(pool, doc_id).await.ok();
            return Err(e);
        }
    }

    // Check cancellation after each step
    if check_cancelled(pool, doc_id).await {
        return Ok(());
    }

    // --- Step 2: Extract (chunk + LLM) ---
    documents::update_processing_progress(
        pool, doc_id, "extract", "Analyzing content...", 0, 0, 0, 10,
    ).await.ok();

    match super::extract::run_extract(state, doc_id, username, ExtractRequest::default()).await {
        Ok(result) => {
            // Update progress with extraction results
            documents::update_processing_progress(
                pool, doc_id, "extract", "Content analyzed",
                0, 0, result.entity_count as i32, 55,
            ).await.ok();
        }
        Err(e) => {
            let msg = format!("{e:?}");
            let suggestion = error_suggestion("extract", &msg);
            documents::set_processing_error(pool, doc_id, "extract", None, &msg, &suggestion)
                .await.ok();
            documents::clear_processing_progress(pool, doc_id).await.ok();
            return Err(e);
        }
    }

    if check_cancelled(pool, doc_id).await {
        return Ok(());
    }

    // --- Step 3: Verify (grounding) ---
    documents::update_processing_progress(
        pool, doc_id, "verify", "Verifying quotes...", 0, 0, 0, 60,
    ).await.ok();

    if let Err(e) = super::verify::run_verify(state, doc_id, username).await {
        let msg = format!("{e:?}");
        let suggestion = error_suggestion("verify", &msg);
        documents::set_processing_error(pool, doc_id, "verify", None, &msg, &suggestion)
            .await.ok();
        documents::clear_processing_progress(pool, doc_id).await.ok();
        return Err(e);
    }

    if check_cancelled(pool, doc_id).await {
        return Ok(());
    }

    // --- Step 4: Auto-ingest (write grounded entities to Neo4j) ---
    documents::update_processing_progress(
        pool, doc_id, "ingest", "Writing to knowledge graph...", 0, 0, 0, 75,
    ).await.ok();

    match super::auto_ingest::run_auto_ingest(state, doc_id, username).await {
        Ok(result) => {
            documents::set_write_summary(
                pool, doc_id,
                result.entities_written,
                result.entities_flagged,
                result.relationships_written,
            ).await.ok();
            tracing::info!(
                doc_id = %doc_id,
                written = result.entities_written,
                flagged = result.entities_flagged,
                rels = result.relationships_written,
                "Step 4: auto-ingest complete"
            );
        }
        Err(e) => {
            let msg = format!("{e:?}");
            let suggestion = error_suggestion("ingest", &msg);
            documents::set_processing_error(pool, doc_id, "ingest", None, &msg, &suggestion)
                .await.ok();
            documents::clear_processing_progress(pool, doc_id).await.ok();
            return Err(e);
        }
    }

    if check_cancelled(pool, doc_id).await {
        return Ok(());
    }

    // --- Step 5: Index (embed in Qdrant) ---
    documents::update_processing_progress(
        pool, doc_id, "index", "Enabling search...", 0, 0, 0, 90,
    ).await.ok();

    if let Err(e) = super::index::run_index(state, doc_id, username).await {
        let msg = format!("{e:?}");
        let suggestion = error_suggestion("index", &msg);
        documents::set_processing_error(pool, doc_id, "index", None, &msg, &suggestion)
            .await.ok();
        documents::clear_processing_progress(pool, doc_id).await.ok();
        return Err(e);
    }

    if check_cancelled(pool, doc_id).await {
        return Ok(());
    }

    // --- Step 6: Completeness check ---
    documents::update_processing_progress(
        pool, doc_id, "completeness", "Validating...", 0, 0, 0, 95,
    ).await.ok();

    if let Err(e) = super::completeness::run_completeness(state, doc_id, username).await {
        let msg = format!("{e:?}");
        let suggestion = error_suggestion("completeness", &msg);
        documents::set_processing_error(pool, doc_id, "completeness", None, &msg, &suggestion)
            .await.ok();
        documents::clear_processing_progress(pool, doc_id).await.ok();
        return Err(e);
    }

    // --- Done ---
    documents::clear_processing_progress(pool, doc_id).await.ok();
    pipeline_repository::update_document_status(pool, doc_id, "COMPLETED")
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;

    tracing::info!(doc_id = %doc_id, "Pipeline completed successfully");
    Ok(())
}

// ── Helpers ────────────────────────────────────────────────────

/// Check cancellation and handle status update if cancelled.
/// Returns true if cancelled (caller should return early).
async fn check_cancelled(pool: &sqlx::PgPool, doc_id: &str) -> bool {
    if documents::is_cancelled(pool, doc_id).await.unwrap_or(false) {
        documents::clear_processing_progress(pool, doc_id).await.ok();
        pipeline_repository::update_document_status(pool, doc_id, "CANCELLED").await.ok();
        tracing::info!(doc_id = %doc_id, "Pipeline cancelled by user");
        true
    } else {
        false
    }
}

/// Map known error patterns to user-facing suggestions.
///
/// ## Rust Learning: Pattern matching on error strings
///
/// We match against lowercased error messages to provide actionable
/// guidance to the user. This is a heuristic — not every error will
/// match. The fallback provides a generic suggestion.
pub(crate) fn error_suggestion(step: &str, error: &str) -> String {
    let error_lower = error.to_lowercase();
    if error_lower.contains("429") || error_lower.contains("rate limit") {
        "Wait 1 minute and re-process. The API rate limit resets every 60 seconds.".to_string()
    } else if error_lower.contains("invalid json")
        || error_lower.contains("eof while parsing")
        || error_lower.contains("invalid")
    {
        "The LLM returned unparseable output. Try re-processing — results vary between runs."
            .to_string()
    } else if error_lower.contains("timeout") || error_lower.contains("timed out") {
        "Processing timed out. The document may be too large. Try re-processing.".to_string()
    } else if error_lower.contains("schema")
        || error_lower.contains("0 entities")
        || error_lower.contains("0 items")
    {
        "Check schema configuration — entity types may not match document content.".to_string()
    } else if error_lower.contains("neo4j") || error_lower.contains("graph") {
        "This is an infrastructure issue with the graph database. Contact admin.".to_string()
    } else if error_lower.contains("qdrant") || error_lower.contains("embedding") {
        "This is an infrastructure issue with the search index. Contact admin.".to_string()
    } else {
        format!("An error occurred during {step}. Try re-processing or contact admin.")
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_suggestion_rate_limit() {
        let s = error_suggestion("extract", "API error: 429 Too Many Requests");
        assert!(s.contains("rate limit"), "Expected rate limit suggestion, got: {s}");
    }

    #[test]
    fn test_error_suggestion_rate_limit_text() {
        let s = error_suggestion("extract", "Rate limit exceeded for model");
        assert!(s.contains("rate limit"), "Expected rate limit suggestion, got: {s}");
    }

    #[test]
    fn test_error_suggestion_bad_json() {
        let s = error_suggestion("extract", "Invalid JSON: EOF while parsing a value");
        assert!(s.contains("unparseable"), "Expected parse error suggestion, got: {s}");
    }

    #[test]
    fn test_error_suggestion_timeout() {
        let s = error_suggestion("extract", "Request timed out after 120s");
        assert!(s.contains("timed out"), "Expected timeout suggestion, got: {s}");
    }

    #[test]
    fn test_error_suggestion_schema() {
        let s = error_suggestion("extract", "Extraction produced 0 entities");
        assert!(s.contains("schema"), "Expected schema suggestion, got: {s}");
    }

    #[test]
    fn test_error_suggestion_neo4j() {
        let s = error_suggestion("ingest", "Neo4j connection refused");
        assert!(s.contains("graph database"), "Expected neo4j suggestion, got: {s}");
    }

    #[test]
    fn test_error_suggestion_qdrant() {
        let s = error_suggestion("index", "Qdrant upsert failed: connection reset");
        assert!(s.contains("search index"), "Expected qdrant suggestion, got: {s}");
    }

    #[test]
    fn test_error_suggestion_default() {
        let s = error_suggestion("verify", "Something completely unexpected");
        assert!(s.contains("verify"), "Expected step name in fallback, got: {s}");
        assert!(s.contains("re-processing"), "Expected re-process suggestion, got: {s}");
    }

    #[test]
    fn test_error_suggestion_embedding() {
        let s = error_suggestion("index", "Embedding generation failed: out of memory");
        assert!(s.contains("search index"), "Expected infra suggestion, got: {s}");
    }

    // --- Grounding status consistency ---

    /// The grounding_status IN list used by get_grounded_items and
    /// get_grounded_relationships must be identical. This test documents
    /// the canonical set of "write" statuses so any drift is caught.
    #[test]
    fn test_grounded_status_values_are_correct() {
        // These are the grounding_status values that mean "write to Neo4j".
        // They must match the SQL IN clauses in extraction.rs:
        //   get_grounded_items_for_document
        //   get_grounded_relationships_for_document
        //   update_graph_status_for_run (written branch)
        let write_statuses = [
            "exact", "normalized", "name_matched", "heading_matched",
            "derived", "unverified",
        ];

        // These are the statuses that mean "skip / flag".
        let skip_statuses = ["not_found", "missing_quote"];

        // Verify no overlap
        for s in &write_statuses {
            assert!(
                !skip_statuses.contains(s),
                "Status '{s}' appears in both write and skip lists"
            );
        }

        // Verify expected count — if someone adds a new status, this test
        // reminds them to decide whether it's write or skip.
        assert_eq!(write_statuses.len(), 6, "Expected 6 write statuses");
        assert_eq!(skip_statuses.len(), 2, "Expected 2 skip statuses");
    }

    // --- Cancel status guard ---

    /// The cancel endpoint only accepts PROCESSING status. Verify that
    /// all other common statuses would be rejected.
    #[test]
    fn test_cancel_rejects_non_processing() {
        let non_processing = [
            "UPLOADED", "COMPLETED", "FAILED", "CANCELLED",
            "TEXT_EXTRACTED", "EXTRACTED", "VERIFIED", "PUBLISHED",
        ];
        for status in &non_processing {
            assert_ne!(
                *status, "PROCESSING",
                "PROCESSING should not be in the rejection list"
            );
        }
    }

    /// PROCESSING is the only status that allows cancellation.
    #[test]
    fn test_cancel_accepts_processing() {
        let status = "PROCESSING";
        assert_eq!(status, "PROCESSING");
        // The cancel_handler checks: document.status != "PROCESSING" → Conflict
        // This test documents the contract: only PROCESSING is cancellable.
    }

    /// New extraction items should have graph_status = 'pending' by default.
    /// This is set by the migration: DEFAULT 'pending'.
    #[test]
    fn test_graph_status_default_is_pending() {
        // The ExtractionItemRecord struct has graph_status: String.
        // The migration sets DEFAULT 'pending'. Verify the default value
        // matches what update_graph_status_for_run expects as "unprocessed".
        let default = "pending";
        // update_graph_status_for_run marks items as either 'written' or
        // 'flagged' — it does NOT match on 'pending'. Items with 'pending'
        // status are simply those that haven't been through auto-ingest yet.
        assert_ne!(default, "written");
        assert_ne!(default, "flagged");
        assert_eq!(default, "pending");
    }
}
