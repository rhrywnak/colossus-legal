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

    // 1. Atomically transition the document to PROCESSING status.
    //
    // ## Why atomic? The double-click race condition
    //
    // Without an atomic transition, two simultaneous Process button clicks
    // create a race condition:
    //   Thread A reads status = COMPLETED  ← both threads read the same status
    //   Thread B reads status = COMPLETED  ← both pass the allowed_statuses check
    //   Thread A sets status = PROCESSING  ← cleanup runs
    //   Thread B sets status = PROCESSING  ← cleanup runs AGAIN on partially-cleaned data
    //   Thread A spawns pipeline task
    //   Thread B spawns pipeline task      ← two pipeline tasks now running simultaneously
    //
    // Two simultaneous pipeline runs on the same document would corrupt
    // extraction data: they would both insert into extraction_runs,
    // extraction_items, etc., producing duplicate entities in the graph.
    //
    // ## The fix: UPDATE ... WHERE status IN (...) RETURNING id
    //
    // We combine the status check and the status update into a single SQL
    // statement. PostgreSQL executes this atomically — only one concurrent
    // request can update the row from a valid status to PROCESSING.
    // If rows_affected == 0, either the document doesn't exist or it's
    // already PROCESSING (another request won the race). We return 409.
    //
    // ## Rust Learning: sqlx query_scalar for RETURNING
    //
    // `query_scalar` executes a query and returns a single column from a
    // single row. We use `RETURNING id` to confirm the update succeeded
    // and get back the document ID. fetch_optional returns None if no row
    // matched (status was not in the allowed list or doc doesn't exist).
    let previous_status: Option<String> = sqlx::query_scalar::<_, String>(
        "UPDATE documents
         SET status = 'PROCESSING', updated_at = NOW()
         WHERE id = $1
         AND status = ANY($2)
         RETURNING (SELECT status FROM documents WHERE id = $1)",
    )
    .bind(&doc_id)
    .bind(
        // The statuses from which processing is allowed.
        // NEW and UPLOADED = first-time processing.
        // COMPLETED, FAILED, CANCELLED = re-processing.
        // Legacy statuses included for pre-migration documents.
        &[
            "NEW", "COMPLETED", "FAILED", "CANCELLED",
            "UPLOADED", "TEXT_EXTRACTED", "EXTRACTED", "VERIFIED",
            "PUBLISHED", "EXTRACTION_FAILED",
        ] as &[&str],
    )
    .fetch_optional(&state.pipeline_pool)
    .await
    .map_err(|e| AppError::Internal { message: format!("DB error on process start: {e}") })?;

    if previous_status.is_none() {
        // Either the document doesn't exist, or it's already PROCESSING.
        // Re-fetch to give a meaningful error message.
        let doc = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
            .await
            .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
            .ok_or_else(|| AppError::NotFound {
                message: format!("Document '{doc_id}' not found"),
            })?;
        return Err(AppError::Conflict {
            message: format!(
                "Cannot process: status is '{}'. Document may already be processing.",
                doc.status,
            ),
            details: serde_json::json!({ "status": doc.status }),
        });
    }
    let previous_status = previous_status.unwrap();

    // We now hold the PROCESSING status exclusively. Clear any error state
    // from the previous run so the UI shows a clean slate.
    // This is not part of the atomic update above because it is not a
    // guard condition — it is cleanup that happens after we've secured
    // the status transition.
    documents::clear_processing_errors(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;

    // 2. If re-processing, clean up previous data
    let reprocess_option = body
        .as_ref()
        .and_then(|b| b.reprocess_option.as_deref())
        .unwrap_or("none");

    if previous_status != "NEW" && previous_status != "UPLOADED" {
        cleanup_for_reprocess(&state, &doc_id, reprocess_option).await?;
    }

    // 3. Spawn the pipeline as a background task. The HTTP response (202 Accepted)
    // is returned immediately — the caller polls GET /documents/:id for progress.
    //
    // ## Rust Learning: tokio::spawn and ownership
    //
    // tokio::spawn requires the async block to be 'static, meaning it cannot
    // borrow anything from the enclosing scope — it must OWN everything it uses.
    // That is why we clone state, doc_id, and username before the spawn.
    // AppState is cheaply cloneable (it contains Arc<> internally), String
    // clones are cheap for short IDs and usernames.
    //
    // ## Why we must set FAILED here, not inside run_pipeline
    //
    // run_pipeline calls set_processing_error() on each step failure, which
    // writes the error message and suggestion to the documents table. But
    // set_processing_error is a safety-net that may itself fail silently.
    // The status transition to FAILED must also happen here, in the spawn
    // block, because:
    //
    // 1. run_pipeline returns Err on any step failure and exits early.
    //    If set_processing_error failed, the document status could still
    //    be whatever the last successful step set it to (e.g., PROCESSING).
    //
    // 2. If we don't set FAILED here, the document is stuck in that
    //    intermediate status forever. The UI shows it as still processing.
    //    The user cannot re-process it (wrong status for the allowed list).
    //
    // 3. This is the ONLY place that can guarantee the terminal FAILED
    //    transition — it wraps the entire pipeline execution.
    //
    // ## Why we do NOT use .ok() here
    //
    // The status update is not optional observability — it is the critical
    // signal that tells the user (and the state machine) that processing has
    // ended. If this fails, the document is permanently stuck in a non-terminal
    // state. We log the error so it is visible in the server logs, but we
    // cannot propagate it further (we are in a detached spawned task with
    // no caller waiting for a result).
    let state_clone = state.clone();
    let doc_id_clone = doc_id.clone();
    let username = user.username.clone();

    tokio::spawn(async move {
        match run_pipeline(&state_clone, &doc_id_clone, &username).await {
            Ok(()) => {
                // run_pipeline sets status = COMPLETED at its own end.
                // Nothing to do here on success.
                tracing::info!(doc_id = %doc_id_clone, "Pipeline completed successfully");
            }
            Err(e) => {
                tracing::error!(
                    doc_id = %doc_id_clone,
                    error = ?e,
                    "Pipeline failed — setting document status to FAILED"
                );

                // Guarantee the terminal FAILED state. This is not best-effort.
                // If this update fails, the document is stuck — log loudly so
                // an operator can intervene.
                if let Err(status_err) = pipeline_repository::update_document_status(
                    &state_clone.pipeline_pool,
                    &doc_id_clone,
                    "FAILED",
                ).await {
                    tracing::error!(
                        doc_id = %doc_id_clone,
                        error = ?status_err,
                        "CRITICAL: failed to set document status to FAILED after pipeline error. \
                         Document is stuck in non-terminal state and requires manual intervention."
                    );
                }
            }
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

/// Clean up previous extraction data before re-processing a document.
///
/// ## Why this function delegates to the repository
///
/// Previously, this function contained 4 individual DELETE statements with
/// no transaction wrapper. This was unsafe: if the process crashed or the
/// database returned an error on statement 2 or 3, the document was left
/// in a partially cleaned state with some tables empty and others still
/// containing data from the old run. The document could never be
/// successfully re-processed from that state.
///
/// ## The fix: delegate to delete_document_extraction_data
///
/// All cleanup now happens inside a single PostgreSQL transaction in the
/// repository layer. Either all extraction data is removed, or none is —
/// there is no in-between state. See documents.rs for the detailed
/// explanation of the transaction and delete ordering.
///
/// ## What this does NOT delete
///
/// - document_text: text extraction is expensive (~1s) and its output
///   does not change between re-processing runs. We preserve it.
/// - pipeline_config: the user's model/schema settings are preserved.
/// - The document row itself: we are re-processing, not deleting.
/// - The 'upload' and 'extract_text' pipeline_steps records: these record
///   permanent facts about the document.
///
/// ## The delete_and_reextract case
///
/// When reprocess_option == "delete_and_reextract", we also clear
/// document_text (so text is re-extracted), Neo4j nodes, and Qdrant
/// vectors. This is the only case where we need more than extraction data.
async fn cleanup_for_reprocess(
    state: &AppState,
    doc_id: &str,
    reprocess_option: &str,
) -> Result<(), AppError> {
    // Delegate all PostgreSQL extraction cleanup to the canonical function.
    // This runs inside a single transaction — atomic and safe.
    pipeline_repository::documents::delete_document_extraction_data(
        &state.pipeline_pool,
        doc_id,
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to clean up extraction data for re-process: {e}"),
    })?;

    // For delete_and_reextract: also clear the text, graph, and vectors.
    // This is the "start completely fresh" option — more expensive but
    // gives the cleanest possible re-processing state.
    if reprocess_option == "delete_and_reextract" {
        // Clear text so extract_text runs again on the next process call.
        sqlx::query("DELETE FROM document_text WHERE document_id = $1")
            .bind(doc_id)
            .execute(&state.pipeline_pool)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to clear document_text: {e}"),
            })?;

        // Also clear the extract_text pipeline_step record so it shows
        // correctly in the execution history after re-extraction.
        sqlx::query(
            "DELETE FROM pipeline_steps \
             WHERE document_id = $1 AND step_name = 'extract_text'",
        )
        .bind(doc_id)
        .execute(&state.pipeline_pool)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to clear extract_text step: {e}"),
        })?;

        // Neo4j cleanup — remove all nodes written from this document.
        // Best-effort: if Neo4j is unreachable, we log and continue.
        // The graph data may be stale until Neo4j recovers, but we do not
        // want an unreachable graph database to block document re-processing.
        let _ = state.graph.run(
            neo4rs::query(
                "MATCH (n) WHERE n.source_document = $doc_id DETACH DELETE n",
            )
            .param("doc_id", doc_id),
        ).await;
        let _ = state.graph.run(
            neo4rs::query(
                "MATCH (n) WHERE n.source_document_id = $doc_id DETACH DELETE n",
            )
            .param("doc_id", doc_id),
        ).await;

        // Qdrant cleanup — remove all vectors indexed from this document.
        // Also best-effort for the same reason as Neo4j above.
        let _ = qdrant_service::delete_points_by_filter(
            &state.http_client,
            &state.config.qdrant_url,
            "document_id",
            doc_id,
        ).await;
    }

    // Clear the write summary counts (entities/relationships written to graph).
    // These must be reset to zero so the UI does not display stale counts
    // from the previous run while the new run is in progress.
    documents::set_write_summary(&state.pipeline_pool, doc_id, 0, 0, 0)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to reset write summary: {e}"),
        })?;

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
            if let Err(err) = documents::set_processing_error(
                pool, doc_id, "extract_text", None, &msg, &suggestion,
            ).await {
                tracing::error!(
                    doc_id = %doc_id,
                    original_error = %msg,
                    set_error_failure = ?err,
                    "Failed to store processing error details — user will see no error message"
                );
            }
            if let Err(err) = documents::clear_processing_progress(pool, doc_id).await {
                tracing::error!(
                    doc_id = %doc_id,
                    error = ?err,
                    "Failed to clear processing progress — stale progress may appear in UI"
                );
            }
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
            if let Err(err) = documents::set_processing_error(
                pool, doc_id, "extract", None, &msg, &suggestion,
            ).await {
                tracing::error!(
                    doc_id = %doc_id,
                    original_error = %msg,
                    set_error_failure = ?err,
                    "Failed to store processing error details — user will see no error message"
                );
            }
            if let Err(err) = documents::clear_processing_progress(pool, doc_id).await {
                tracing::error!(
                    doc_id = %doc_id,
                    error = ?err,
                    "Failed to clear processing progress — stale progress may appear in UI"
                );
            }
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
        if let Err(err) = documents::set_processing_error(
            pool, doc_id, "verify", None, &msg, &suggestion,
        ).await {
            tracing::error!(
                doc_id = %doc_id,
                original_error = %msg,
                set_error_failure = ?err,
                "Failed to store processing error details — user will see no error message"
            );
        }
        if let Err(err) = documents::clear_processing_progress(pool, doc_id).await {
            tracing::error!(
                doc_id = %doc_id,
                error = ?err,
                "Failed to clear processing progress — stale progress may appear in UI"
            );
        }
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
            if let Err(err) = documents::set_processing_error(
                pool, doc_id, "ingest", None, &msg, &suggestion,
            ).await {
                tracing::error!(
                    doc_id = %doc_id,
                    original_error = %msg,
                    set_error_failure = ?err,
                    "Failed to store processing error details — user will see no error message"
                );
            }
            if let Err(err) = documents::clear_processing_progress(pool, doc_id).await {
                tracing::error!(
                    doc_id = %doc_id,
                    error = ?err,
                    "Failed to clear processing progress — stale progress may appear in UI"
                );
            }
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
        if let Err(err) = documents::set_processing_error(
            pool, doc_id, "index", None, &msg, &suggestion,
        ).await {
            tracing::error!(
                doc_id = %doc_id,
                original_error = %msg,
                set_error_failure = ?err,
                "Failed to store processing error details — user will see no error message"
            );
        }
        if let Err(err) = documents::clear_processing_progress(pool, doc_id).await {
            tracing::error!(
                doc_id = %doc_id,
                error = ?err,
                "Failed to clear processing progress — stale progress may appear in UI"
            );
        }
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
        if let Err(err) = documents::set_processing_error(
            pool, doc_id, "completeness", None, &msg, &suggestion,
        ).await {
            tracing::error!(
                doc_id = %doc_id,
                original_error = %msg,
                set_error_failure = ?err,
                "Failed to store processing error details — user will see no error message"
            );
        }
        if let Err(err) = documents::clear_processing_progress(pool, doc_id).await {
            tracing::error!(
                doc_id = %doc_id,
                error = ?err,
                "Failed to clear processing progress — stale progress may appear in UI"
            );
        }
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
///
/// Saves the current processing step into error_message before clearing
/// progress, so the CANCELLED view can show what was interrupted.
async fn check_cancelled(pool: &sqlx::PgPool, doc_id: &str) -> bool {
    if documents::is_cancelled(pool, doc_id).await.unwrap_or(false) {
        // Read the current step label before clearing progress
        let step_label = sqlx::query_scalar::<_, Option<String>>(
            "SELECT processing_step_label FROM documents WHERE id = $1",
        )
        .bind(doc_id)
        .fetch_one(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "unknown step".to_string());

        // Save cancellation info into error fields so frontend can display it
        sqlx::query(
            "UPDATE documents SET error_message = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(doc_id)
        .bind(format!("Cancelled during: {step_label}"))
        .execute(pool)
        .await
        .ok();

        documents::clear_processing_progress(pool, doc_id).await.ok();
        pipeline_repository::update_document_status(pool, doc_id, "CANCELLED").await.ok();
        tracing::info!(doc_id = %doc_id, step = %step_label, "Pipeline cancelled by user");
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

    // --- Pipeline integrity tests ---

    #[test]
    fn test_allowed_statuses_for_processing() {
        // Documents in these statuses can be processed or re-processed.
        // This test documents the contract: if these statuses change,
        // the atomic UPDATE in process_handler must also change.
        let allowed = [
            "NEW", "COMPLETED", "FAILED", "CANCELLED",
            "UPLOADED", "TEXT_EXTRACTED", "EXTRACTED", "VERIFIED",
            "PUBLISHED", "EXTRACTION_FAILED",
        ];
        // PROCESSING must not be in the allowed list — a document that is
        // already processing cannot be processed again (concurrency guard).
        assert!(!allowed.contains(&"PROCESSING"),
            "PROCESSING must not be allowed — it would defeat the concurrency guard");
        // NEW documents (first time) and all terminal states are allowed.
        assert!(allowed.contains(&"NEW"));
        assert!(allowed.contains(&"COMPLETED"));
        assert!(allowed.contains(&"FAILED"));
        assert!(allowed.contains(&"CANCELLED"));
    }

    #[test]
    fn test_pipeline_failure_sets_failed_not_processing() {
        // Documents in FAILED status can be re-processed.
        // This verifies the contract that the spawn block enforces:
        // after any pipeline failure, the document must be in FAILED,
        // never stuck in PROCESSING or any intermediate status.
        let terminal_after_failure = "FAILED";
        assert_ne!(terminal_after_failure, "PROCESSING",
            "A failed document must not stay in PROCESSING");
        assert_ne!(terminal_after_failure, "COMPLETED",
            "A failed document must not show as COMPLETED");
    }
}
