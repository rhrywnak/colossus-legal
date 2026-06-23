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

/// Widened relationships-DELETE: clears every `extraction_relationships` row
/// that *touches* this document, not merely the rows it owns.
///
/// ## Why three predicates, not one (DELETE-FK-FIX)
///
/// A relationship row references three things: the owning `document_id`, and
/// two `extraction_items` endpoints (`from_item_id`, `to_item_id`), both under
/// a RESTRICT foreign key (`extraction_relationships_from_item_id_fkey` /
/// `..._to_item_id_fkey`, PostgreSQL default-named — the migration declares the
/// FKs inline with no `CONSTRAINT` name). Since the allegation-anchored
/// REBUTS/CONTRADICTS work (commit `60b9131`), one document can own a
/// relationship whose endpoint is an item belonging to a *different* document.
///
/// The old predicate (`WHERE document_id = $1`) deleted only owned rows. When
/// another document owned an edge pointing AT this document's items, that edge
/// survived; the subsequent `DELETE FROM extraction_items` then tripped the
/// RESTRICT FK and rolled the whole transaction back. That is the defect that
/// blocked single-document deletes three times (worked around 2026-06-18 by
/// deleting George and CFS together).
///
/// The fix matches rows on either FK endpoint as well as ownership.
///
/// ## Rust Learning: subquery `IN (SELECT …)` vs. `IN ($1, $2, …)` vs. `= ANY`
///
/// We want "every relationship whose endpoint is one of this document's item
/// ids". The tempting Rust move — fetch the ids into a `Vec<i32>` and splice
/// them into `IN ($1, $2, …)` — does NOT work with sqlx: sqlx binds one
/// positional parameter per `$n` and will not expand a `Vec` into a Postgres
/// IN-list. The Postgres-native way to bind a collection is `= ANY($1::int[])`
/// with the slice bound as a single array parameter (`&[i32]`, borrowed for the
/// bind and dropped after `execute` — no move needed).
///
/// Here we use neither: a correlated **subquery** keeps the id set inside the
/// one SQL statement. That is strictly better for this case — it runs in a
/// single round-trip (no SELECT-then-DELETE), stays atomic inside the caller's
/// transaction, needs no Rust-side `Vec<i32>` to materialise, and matches the
/// `review_edit_history` delete a few lines below that already uses this idiom.
/// `$1` is bound once (a borrowed `&str`) and reused by all three predicates.
const DELETE_RELATIONSHIPS_TOUCHING_DOCUMENT: &str = "DELETE FROM extraction_relationships \
     WHERE document_id = $1 \
        OR from_item_id IN (SELECT id FROM extraction_items WHERE document_id = $1) \
        OR to_item_id IN (SELECT id FROM extraction_items WHERE document_id = $1)";

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

    // Relationships reference both extraction_items and extraction_runs. We
    // must clear rows this document OWNS *and* rows another document owns that
    // point at this document's items, or the RESTRICT FK on the item endpoints
    // blocks the extraction_items delete below. See
    // `DELETE_RELATIONSHIPS_TOUCHING_DOCUMENT` for the full rationale.
    sqlx::query(DELETE_RELATIONSHIPS_TOUCHING_DOCUMENT)
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

#[cfg(test)]
mod tests {
    //! SQL-shape tests. There is no `#[sqlx::test]` / live-DB harness in this
    //! repo, so the FK widening is verified here by asserting the generated
    //! SQL matches relationships on BOTH foreign-key endpoints — the exact
    //! property whose absence caused the recurring delete failure. The
    //! end-to-end behaviour (a delete of a document whose items are targeted by
    //! another document's relationship succeeds) is verified manually on DEV.
    use super::*;

    /// The widened DELETE must constrain on the owning `document_id` AND on
    /// each item-endpoint FK. If a future edit narrows it back to only
    /// `document_id`, this test fails and names the regression.
    #[test]
    fn delete_relationships_sql_covers_both_fk_endpoints() {
        let sql = DELETE_RELATIONSHIPS_TOUCHING_DOCUMENT;
        assert!(
            sql.contains("document_id = $1"),
            "must still clear rows this document owns"
        );
        assert!(
            sql.contains(
                "from_item_id IN (SELECT id FROM extraction_items WHERE document_id = $1)"
            ),
            "must clear rows pointing FROM this document's items (from_item_id FK)"
        );
        assert!(
            sql.contains("to_item_id IN (SELECT id FROM extraction_items WHERE document_id = $1)"),
            "must clear rows pointing TO this document's items (to_item_id FK) — \
             the endpoint that caused the recurring RESTRICT rollback"
        );
    }
}
