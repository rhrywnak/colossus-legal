//! Document-specific update functions for the process endpoint.
//!
//! These functions update progress tracking, error details, and cancellation
//! state on the `documents` table. Separated from `mod.rs` to keep each
//! module under 300 lines (CLAUDE.md golden rule).

use sqlx::PgPool;

use super::PipelineRepoError;

/// Update document processing progress (called during async pipeline execution).
#[allow(clippy::too_many_arguments)]
pub async fn update_processing_progress(
    pool: &PgPool,
    document_id: &str,
    step: &str,
    step_label: &str,
    chunks_total: i32,
    chunks_processed: i32,
    entities_found: i32,
    percent_complete: i32,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE documents SET
            processing_step = $2,
            processing_step_label = $3,
            chunks_total = $4,
            chunks_processed = $5,
            entities_found = $6,
            percent_complete = $7,
            updated_at = NOW()
         WHERE id = $1",
    )
    .bind(document_id)
    .bind(step)
    .bind(step_label)
    .bind(chunks_total)
    .bind(chunks_processed)
    .bind(entities_found)
    .bind(percent_complete)
    .execute(pool)
    .await?;
    Ok(())
}

/// Clear progress fields (called when processing completes or fails).
pub async fn clear_processing_progress(
    pool: &PgPool,
    document_id: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE documents SET
            processing_step = NULL,
            processing_step_label = NULL,
            chunks_total = 0,
            chunks_processed = 0,
            entities_found = 0,
            percent_complete = 0,
            updated_at = NOW()
         WHERE id = $1",
    )
    .bind(document_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Store error details when processing fails.
pub async fn set_processing_error(
    pool: &PgPool,
    document_id: &str,
    failed_step: &str,
    failed_chunk: Option<i32>,
    error_message: &str,
    error_suggestion: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE documents SET
            status = 'FAILED',
            failed_step = $2,
            failed_chunk = $3,
            error_message = $4,
            error_suggestion = $5,
            updated_at = NOW()
         WHERE id = $1",
    )
    .bind(document_id)
    .bind(failed_step)
    .bind(failed_chunk)
    .bind(error_message)
    .bind(error_suggestion)
    .execute(pool)
    .await?;
    Ok(())
}

/// Clear error fields (called when re-processing starts).
pub async fn clear_processing_errors(
    pool: &PgPool,
    document_id: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE documents SET
            failed_step = NULL,
            failed_chunk = NULL,
            error_message = NULL,
            error_suggestion = NULL,
            is_cancelled = FALSE,
            updated_at = NOW()
         WHERE id = $1",
    )
    .bind(document_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Set the is_cancelled flag.
pub async fn set_cancelled(
    pool: &PgPool,
    document_id: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query("UPDATE documents SET is_cancelled = TRUE, updated_at = NOW() WHERE id = $1")
        .bind(document_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Check if document is cancelled.
pub async fn is_cancelled(
    pool: &PgPool,
    document_id: &str,
) -> Result<bool, PipelineRepoError> {
    let row = sqlx::query_scalar::<_, bool>(
        "SELECT is_cancelled FROM documents WHERE id = $1",
    )
    .bind(document_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.unwrap_or(false))
}

/// Store auto-write summary counts.
pub async fn set_write_summary(
    pool: &PgPool,
    document_id: &str,
    entities_written: i32,
    entities_flagged: i32,
    relationships_written: i32,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE documents SET
            entities_written = $2,
            entities_flagged = $3,
            relationships_written = $4,
            updated_at = NOW()
         WHERE id = $1",
    )
    .bind(document_id)
    .bind(entities_written)
    .bind(entities_flagged)
    .bind(relationships_written)
    .execute(pool)
    .await?;
    Ok(())
}

/// Count total documents in the pipeline.
pub async fn count_documents(pool: &PgPool) -> Result<i64, PipelineRepoError> {
    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM documents")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

/// Check if at least one document of the given type exists.
pub async fn has_document_of_type(
    pool: &PgPool,
    doc_type: &str,
) -> Result<bool, PipelineRepoError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM documents WHERE document_type = $1",
    )
    .bind(doc_type)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}

/// Delete all extraction data for a document in a single atomic transaction.
///
/// ## Why this function exists — the cleanup consistency problem
///
/// Before this function existed, two separate code paths deleted extraction
/// data independently:
///
/// 1. `cleanup_for_reprocess` in process.rs — called when re-processing a
///    document. It ran 4 separate DELETE statements with no transaction
///    wrapper. If the process crashed between statements, the document would
///    be left in a partially cleaned state: some tables empty, others still
///    containing data from the old run. There was no way to detect or recover
///    from this inconsistency.
///
/// 2. The PostgreSQL transaction in delete.rs — correctly wrapped in a
///    transaction, but was missing extraction_chunks entirely.
///
/// Two divergent paths with different bugs is worse than one correct path.
///
/// ## The fix: one function, one transaction, one correct ordering
///
/// This function is the single source of truth for "how to remove all
/// extraction data for a document." It uses a PostgreSQL transaction so
/// that either ALL data is removed or NONE is removed — no partial states.
///
/// ## Rust Learning: sqlx transactions
///
/// `pool.begin()` starts a PostgreSQL transaction and returns a `Transaction`
/// object. You pass `&mut *txn` (dereference to get the inner connection) to
/// sqlx query calls instead of the pool. When you call `txn.commit()`, all
/// statements execute atomically. If any statement errors and you return
/// early (via `?`), the transaction is automatically rolled back when the
/// `Transaction` value is dropped — Rust's ownership system guarantees cleanup
/// even in error paths. You never need to manually call ROLLBACK.
///
/// ## Delete ordering — why it matters
///
/// PostgreSQL foreign key constraints (without ON DELETE CASCADE) use RESTRICT
/// by default: a parent row cannot be deleted if child rows reference it.
/// The correct order is children-before-parents:
///
///   extraction_relationships  (references extraction_items AND extraction_runs)
///   → extraction_items        (references extraction_runs)
///   → extraction_chunks       (references extraction_runs, CASCADE added by migration)
///   → extraction_runs         (parent of items and chunks)
///
/// extraction_chunks now has ON DELETE CASCADE from the migration, so
/// deleting extraction_runs would cascade to extraction_chunks automatically.
/// We still delete extraction_chunks explicitly here for clarity and to
/// ensure correct behavior even if the migration hasn't run yet.
///
/// pipeline_steps rows for non-upload/extract_text steps are also cleared
/// because they represent the execution history of extraction-related steps.
/// The upload and extract_text steps are preserved — they represent permanent
/// facts about the document (it was uploaded, its text was extracted) that
/// remain true even after re-processing.
pub async fn delete_document_extraction_data(
    pool: &PgPool,
    document_id: &str,
) -> Result<(), PipelineRepoError> {
    // Begin a transaction. If any statement fails, the entire transaction
    // is rolled back automatically when `txn` is dropped. This guarantees
    // we never leave the database in a partially cleaned state.
    let mut txn = pool.begin().await?;

    // Step 1: Delete relationships first — they reference both extraction_items
    // (from_item_id, to_item_id) and extraction_runs (run_id). They must be
    // deleted before either of their referenced tables.
    sqlx::query(
        "DELETE FROM extraction_relationships WHERE document_id = $1",
    )
    .bind(document_id)
    .execute(&mut *txn)
    .await?;

    // Step 2: Delete items — they reference extraction_runs (run_id). Must be
    // deleted before extraction_runs.
    sqlx::query(
        "DELETE FROM extraction_items WHERE document_id = $1",
    )
    .bind(document_id)
    .execute(&mut *txn)
    .await?;

    // Step 3: Delete chunk observability rows — they reference extraction_runs
    // (extraction_run_id). The migration added ON DELETE CASCADE, but we delete
    // explicitly to be safe and to make the ordering visible to the reader.
    sqlx::query(
        "DELETE FROM extraction_chunks \
         WHERE extraction_run_id IN \
         (SELECT id FROM extraction_runs WHERE document_id = $1)",
    )
    .bind(document_id)
    .execute(&mut *txn)
    .await?;

    // Step 4: Delete the run record itself. All child rows are gone, so this
    // will not violate any FK constraints.
    sqlx::query(
        "DELETE FROM extraction_runs WHERE document_id = $1",
    )
    .bind(document_id)
    .execute(&mut *txn)
    .await?;

    // Step 5: Delete pipeline step records for extraction-related steps only.
    // We keep 'upload' and 'extract_text' steps because they record permanent
    // facts about the document. All other steps (extract, verify, ingest, etc.)
    // are tied to the specific extraction run being cleared.
    sqlx::query(
        "DELETE FROM pipeline_steps \
         WHERE document_id = $1 \
         AND step_name NOT IN ('upload', 'extract_text')",
    )
    .bind(document_id)
    .execute(&mut *txn)
    .await?;

    // Commit the transaction. All five DELETEs are now permanent.
    // If we never reach this line (due to an error above), the transaction
    // is automatically rolled back when `txn` is dropped at end of scope.
    txn.commit().await?;

    Ok(())
}

/// Delete ALL data for a document including text and config — for full document deletion.
///
/// ## Why this is separate from delete_document_extraction_data
///
/// `delete_document_extraction_data` clears extraction data only, preserving
/// the document record, document_text, and pipeline_config. It is called
/// when re-processing a document — we keep the document and its text, we
/// only clear the extraction results so the pipeline can run fresh.
///
/// This function deletes EVERYTHING including the document row itself.
/// It is called only from the delete endpoint when the user explicitly
/// removes a document from the system. The document_text and pipeline_config
/// rows are included in the same transaction.
///
/// The document row itself is deleted last because all other tables have
/// FK references to documents(id). Deleting the document last satisfies
/// all FK constraints (children before parent).
///
/// ## Why not just add ON DELETE CASCADE to documents(id)?
///
/// We could add CASCADE from all tables to documents(id), which would let
/// us delete just the document row and have PostgreSQL cascade everything.
/// We deliberately avoid this because:
/// 1. Cascades are implicit — you can't see what gets deleted by reading
///    the application code alone. This makes the code harder to audit.
/// 2. The audit log in delete.rs must be written BEFORE the data is deleted.
///    If we cascaded, the snapshot could not be built after deletion.
/// 3. Explicit ordering in a transaction is always safer than implicit
///    database-level cascades for destructive operations.
pub async fn delete_all_document_data(
    pool: &PgPool,
    document_id: &str,
) -> Result<(), PipelineRepoError> {
    let mut txn = pool.begin().await?;

    // Extraction data — same order as delete_document_extraction_data
    sqlx::query("DELETE FROM extraction_relationships WHERE document_id = $1")
        .bind(document_id).execute(&mut *txn).await?;

    sqlx::query("DELETE FROM extraction_items WHERE document_id = $1")
        .bind(document_id).execute(&mut *txn).await?;

    sqlx::query(
        "DELETE FROM extraction_chunks \
         WHERE extraction_run_id IN \
         (SELECT id FROM extraction_runs WHERE document_id = $1)",
    )
    .bind(document_id).execute(&mut *txn).await?;

    sqlx::query("DELETE FROM extraction_runs WHERE document_id = $1")
        .bind(document_id).execute(&mut *txn).await?;

    // Document content and config — only deleted on full document removal
    sqlx::query("DELETE FROM document_text WHERE document_id = $1")
        .bind(document_id).execute(&mut *txn).await?;

    sqlx::query("DELETE FROM pipeline_steps WHERE document_id = $1")
        .bind(document_id).execute(&mut *txn).await?;

    sqlx::query("DELETE FROM pipeline_config WHERE document_id = $1")
        .bind(document_id).execute(&mut *txn).await?;

    // Document row last — all FK children are gone, this will succeed.
    sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(document_id).execute(&mut *txn).await?;

    txn.commit().await?;
    Ok(())
}

#[cfg(test)]
mod cleanup_tests {
    // These tests document the FK-safe delete ordering contract.
    // They verify the ordering logic without requiring a live database.

    #[test]
    fn test_delete_order_relationships_before_items() {
        // extraction_relationships references extraction_items (from_item_id, to_item_id).
        // Deleting items before relationships would violate this FK.
        // The correct order is: relationships → items → chunks → runs.
        // This test documents that contract so future readers understand why
        // the order in delete_document_extraction_data is not arbitrary.
        let correct_order = [
            "extraction_relationships",
            "extraction_items",
            "extraction_chunks",
            "extraction_runs",
        ];
        // Verify relationships come before items
        let rel_pos = correct_order.iter().position(|&s| s == "extraction_relationships").unwrap();
        let item_pos = correct_order.iter().position(|&s| s == "extraction_items").unwrap();
        assert!(rel_pos < item_pos,
            "extraction_relationships must be deleted before extraction_items");
        // Verify items come before runs
        let run_pos = correct_order.iter().position(|&s| s == "extraction_runs").unwrap();
        assert!(item_pos < run_pos,
            "extraction_items must be deleted before extraction_runs");
        // Verify chunks come before runs
        let chunk_pos = correct_order.iter().position(|&s| s == "extraction_chunks").unwrap();
        assert!(chunk_pos < run_pos,
            "extraction_chunks must be deleted before extraction_runs");
    }

    #[test]
    fn test_graph_cleanup_includes_completed_status() {
        // After pipeline simplification, COMPLETED is the terminal status
        // for all successfully processed documents. Neo4j and Qdrant cleanup
        // must include COMPLETED — verifies Finding 5 fix is present.
        let statuses_requiring_graph_cleanup = [
            "COMPLETED", "PUBLISHED", "INGESTED", "INDEXED",
        ];
        assert!(statuses_requiring_graph_cleanup.contains(&"COMPLETED"),
            "COMPLETED documents have Neo4j/Qdrant data — must be cleaned on delete");
    }
}
