//! Transactional document-deletion paths.
//!
//! Two destructive operations live here, both wrapped in a single
//! sqlx transaction so a partial failure rolls back cleanly:
//!
//! - [`delete_document_extraction_data`] — clear extraction-result
//!   tables for a re-process. Preserves the document row and
//!   `document_text` so the pipeline can re-run against the same
//!   source.
//! - [`delete_all_document_data`] — full document removal including
//!   `document_text`, `pipeline_config`, `pipeline_jobs`, and the
//!   `documents` row itself.
//!
//! Both use explicit DELETE ordering (children before parents) rather
//! than relying on database-level `ON DELETE CASCADE`. The rationale
//! is documented per-function below.

use sqlx::PgPool;

use super::PipelineRepoError;

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
    sqlx::query("DELETE FROM extraction_relationships WHERE document_id = $1")
        .bind(document_id)
        .execute(&mut *txn)
        .await?;

    // Step 2: Delete items — they reference extraction_runs (run_id). Must be
    // deleted before extraction_runs.
    sqlx::query("DELETE FROM extraction_items WHERE document_id = $1")
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
    sqlx::query("DELETE FROM extraction_runs WHERE document_id = $1")
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
        .bind(document_id)
        .execute(&mut *txn)
        .await?;

    sqlx::query("DELETE FROM extraction_items WHERE document_id = $1")
        .bind(document_id)
        .execute(&mut *txn)
        .await?;

    sqlx::query(
        "DELETE FROM extraction_chunks \
         WHERE extraction_run_id IN \
         (SELECT id FROM extraction_runs WHERE document_id = $1)",
    )
    .bind(document_id)
    .execute(&mut *txn)
    .await?;

    sqlx::query("DELETE FROM extraction_runs WHERE document_id = $1")
        .bind(document_id)
        .execute(&mut *txn)
        .await?;

    // Document content and config — only deleted on full document removal
    sqlx::query("DELETE FROM document_text WHERE document_id = $1")
        .bind(document_id)
        .execute(&mut *txn)
        .await?;

    sqlx::query("DELETE FROM pipeline_steps WHERE document_id = $1")
        .bind(document_id)
        .execute(&mut *txn)
        .await?;

    sqlx::query("DELETE FROM pipeline_config WHERE document_id = $1")
        .bind(document_id)
        .execute(&mut *txn)
        .await?;

    // Pipeline jobs — no FK to documents, but `job_key` holds the
    // document_id. Failing to clear these strands orphaned rows behind
    // (usually a FAILED job) and prevents re-uploading the same document
    // because the old job blocks the new insert.
    sqlx::query("DELETE FROM pipeline_jobs WHERE job_key = $1")
        .bind(document_id)
        .execute(&mut *txn)
        .await?;

    // Document row last — all FK children are gone, this will succeed.
    sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(document_id)
        .execute(&mut *txn)
        .await?;

    txn.commit().await?;
    Ok(())
}
