//! Transactional document-deletion path.
//!
//! [`delete_all_document_data`] performs full document removal —
//! extraction-tier rows, `document_text`, `pipeline_config`,
//! `pipeline_jobs`, the document's extracted authored relationships, and
//! the `documents` row itself — wrapped in a single sqlx transaction so a
//! partial failure rolls back cleanly.
//!
//! It uses explicit DELETE ordering (children before parents) rather than
//! relying on database-level `ON DELETE CASCADE`. The rationale is
//! documented per-function below.

use sqlx::PgConnection;
use sqlx::PgPool;

use super::authored_entities::delete_extracted_authored_relationships_for_document;
use super::PipelineRepoError;

/// Delete the extraction-tier rows for a document, children-before-parents,
/// inside the caller's transaction.
///
/// ## Why a shared helper
///
/// Both the historical re-process path and the full-delete path must clear
/// these four tables in the same FK-safe order. Centralising the ordering
/// here means a future table addition (or a missed child like
/// `review_edit_history` was — see below) is fixed in exactly one place.
///
/// ## Delete ordering — why it matters
///
/// PostgreSQL foreign keys without `ON DELETE CASCADE` default to RESTRICT:
/// a parent row cannot be deleted while child rows reference it. The order
/// here is strictly children-before-parents:
///
///   review_edit_history       (references extraction_items.id, RESTRICT)
///   → extraction_relationships (references extraction_items AND extraction_runs)
///   → extraction_items         (references extraction_runs)
///   → extraction_runs          (parent of items)
///
/// `review_edit_history` MUST go first: its `item_id` FK to
/// `extraction_items(id)` is RESTRICT (migration
/// `20260411_f5_review_edit_history.sql`), so deleting items while any
/// history row survives aborts the whole transaction.
///
/// ## Rust Learning: `&mut PgConnection` instead of `&PgPool`
///
/// We take the live transaction connection (`&mut PgConnection`), not the
/// pool. A pool hands out a *fresh* connection per query — those would run
/// outside the caller's transaction and commit independently, defeating
/// atomicity. By threading the caller's `&mut *txn` through, every DELETE
/// here joins the one transaction the caller will later `commit()` (or let
/// roll back on drop). `&mut` because a connection executes one statement
/// at a time and sqlx encodes that exclusivity in the borrow.
async fn delete_extraction_tier_rows(
    conn: &mut PgConnection,
    document_id: &str,
) -> Result<(), PipelineRepoError> {
    // Children first: edit-history rows reference extraction_items(id) under
    // a RESTRICT FK. The subquery resolves the document's item ids before the
    // items themselves are removed below.
    sqlx::query(
        "DELETE FROM review_edit_history WHERE item_id IN \
         (SELECT id FROM extraction_items WHERE document_id = $1)",
    )
    .bind(document_id)
    .execute(&mut *conn)
    .await?;

    // Relationships reference both extraction_items and extraction_runs.
    sqlx::query("DELETE FROM extraction_relationships WHERE document_id = $1")
        .bind(document_id)
        .execute(&mut *conn)
        .await?;

    // Items reference extraction_runs — delete before the runs.
    sqlx::query("DELETE FROM extraction_items WHERE document_id = $1")
        .bind(document_id)
        .execute(&mut *conn)
        .await?;

    // Runs last in this tier — all referencing children are gone.
    sqlx::query("DELETE FROM extraction_runs WHERE document_id = $1")
        .bind(document_id)
        .execute(&mut *conn)
        .await?;

    Ok(())
}

/// Delete ALL data for a document including text and config — for full document deletion.
///
/// ## What it removes
///
/// Everything keyed to the document: the extraction tier (via
/// [`delete_extraction_tier_rows`]), the document's extracted (Pass-2)
/// authored relationships, `document_text`, `pipeline_steps`,
/// `pipeline_config`, `pipeline_jobs`, and finally the `documents` row
/// itself. All in one transaction — either every row goes or none does.
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

    // Extraction tier (review_edit_history → relationships → items → runs),
    // children-before-parents, inside this transaction.
    delete_extraction_tier_rows(&mut txn, document_id).await?;

    // Extracted (Pass-2) authored relationships this document asserted.
    // Scoped to `provenance = 'extracted'` by the called function, so
    // canonical/authored rows (document_id NULL) are never touched. Logging
    // the count gives the delete a distinct observable (Rule 1): "removed N
    // extracted edges" vs. "removed none" are different operational states.
    let extracted_edges_removed =
        delete_extracted_authored_relationships_for_document(&mut *txn, document_id).await?;
    tracing::info!(
        doc_id = %document_id,
        extracted_edges_removed,
        "delete_all_document_data: removed extracted authored relationships"
    );

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
