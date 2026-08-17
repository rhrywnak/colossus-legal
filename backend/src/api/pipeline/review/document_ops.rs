//! Document-level review operations: revert-ingest, reprocess, bulk-approve.

use axum::{extract::Path as AxumPath, extract::State, Json};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::models::document_status::{
    STATUS_INDEXED, STATUS_INGESTED, STATUS_PUBLISHED, STATUS_TEXT_EXTRACTED, STATUS_VERIFIED,
};
use crate::pipeline::workflow_steps::{STEP_EXTRACT_TEXT, STEP_UPLOAD};
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository::{self, review as review_repo, steps};
use crate::state::AppState;

use crate::api::pipeline::delete::{cleanup_neo4j, cleanup_qdrant};

use super::{BulkApproveRequest, BulkApproveResponse, ReprocessResponse, RevertIngestResponse};

// ── Revert Ingest ──────────────────────────────────────────────

/// POST /documents/:id/revert-ingest — remove Neo4j data and reset to VERIFIED.
pub async fn revert_ingest_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<Json<RevertIngestResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST revert-ingest");

    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to fetch document '{doc_id}' for revert-ingest: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    if !matches!(
        document.status.as_str(),
        STATUS_INGESTED | STATUS_INDEXED | STATUS_PUBLISHED
    ) {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot revert ingest: status is '{}', expected {STATUS_INGESTED}, {STATUS_INDEXED}, or {STATUS_PUBLISHED}",
                document.status
            ),
            details: serde_json::json!({"status": document.status}),
        });
    }

    // Remove Neo4j data (reuse delete module's cleanup logic)
    cleanup_neo4j(&state, &doc_id).await;

    // Reset status to VERIFIED
    pipeline_repository::update_document_status(&state.pipeline_pool, &doc_id, STATUS_VERIFIED)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update status: {e}"),
        })?;

    log_admin_action(
        &state.audit_repo,
        &user.username,
        "pipeline.document.revert_ingest",
        Some("document"),
        Some(&doc_id),
        Some(serde_json::json!({"previous_status": document.status})),
    )
    .await;

    tracing::info!(doc_id = %doc_id, previous = %document.status, "Ingest reverted — status → VERIFIED");

    Ok(Json(RevertIngestResponse {
        document_id: doc_id,
        status: STATUS_VERIFIED.to_string(),
        message: "Ingest reverted. Items unlocked for re-review.".to_string(),
    }))
}

/// The single-parameter DELETEs that clear a document's extraction state, in
/// FK-safe order.
///
/// ## Why an ordered const rather than four inline statements
///
/// The ORDER is a correctness property, not a style: `review_edit_history`
/// references `extraction_items`, and `extraction_items` references
/// `extraction_runs`, both with RESTRICT. Reversing any two of these rolls the
/// whole clear back on a foreign-key violation, and the failure would only ever
/// appear against a document that actually had review history. As data it can be
/// asserted by a test without a database; as four statements buried in a
/// function it could only be checked by reading.
///
/// `pipeline_steps` is deliberately NOT here — it takes two extra bind
/// parameters for the step names it preserves, and bending it into this shape
/// would mean either a magic empty bind or losing the `STEP_*` constants.
const CLEAR_STATEMENTS: &[&str] = &[
    // Children first.
    "DELETE FROM review_edit_history WHERE item_id IN \
     (SELECT id FROM extraction_items WHERE document_id = $1)",
    // Both endpoint FKs, not just the owning document — see the constant's doc.
    REPROCESS_DELETE_RELATIONSHIPS_SQL,
    "DELETE FROM extraction_items WHERE document_id = $1",
    // Parent last.
    "DELETE FROM extraction_runs WHERE document_id = $1",
];

/// What to do with `documents.status` once the extraction state is gone.
///
/// `ResetToTextExtracted` is the reprocess handler's answer: it leaves the
/// document parked for a human to press Process. `LeaveUntouched` is the
/// re-extraction path's, because it writes `PROCESSING` itself moments later and
/// a TEXT_EXTRACTED write in between would flicker the UI through a state the
/// document is never actually in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostClearStatus {
    ResetToTextExtracted,
    LeaveUntouched,
}

/// Delete a document's extraction state so a re-extraction starts clean.
///
/// ## Why this is shared rather than owned by the reprocess handler
///
/// Two callers need exactly this, and they must not drift: `reprocess_handler`
/// (which resets the document to TEXT_EXTRACTED and stops) and `process_handler`
/// when the operator asked for a re-extraction (REEXTRACT_PATH). Before this was
/// shared, only the first existed and nothing called it — which is why every
/// "Re-process" in the UI silently skipped both LLM passes: the COMPLETED
/// `extraction_runs` row survived, and both passes short-circuit on it.
///
/// ## What it deletes, and what it deliberately keeps
///
/// Deletes, in FK-safe order: `review_edit_history` for this document's items ·
/// `extraction_relationships` touching them (both endpoint FKs, not just the
/// owning `document_id` — see [`REPROCESS_DELETE_RELATIONSHIPS_SQL`]) ·
/// `extraction_items` · `extraction_runs` · every `pipeline_steps` row except
/// upload and text-extraction.
///
/// KEEPS the upload and `extract_text` steps, and never touches the extracted
/// text itself: re-extraction means running the MODEL again, not re-OCRing a PDF
/// that has not changed. That is also why `extract_text` still reports
/// `already_extracted` on a re-extract run and why only the two LLM steps are
/// expected to burn tokens.
///
/// [`PostClearStatus`] is an enum rather than a `bool` so both call sites read as
/// what they mean: `clear_extraction_state(pool, id, false)` told a reader
/// nothing about which behaviour `false` selected.
///
/// ## Rust Learning: one transaction, many statements
///
/// `pool.begin()` yields a `Transaction` that borrows the connection; every
/// `execute(&mut *txn)` reborrows it. Nothing is durable until `commit()`, so an
/// error at any statement leaves the document exactly as it was — which matters
/// here more than usual, because a half-cleared document would have items whose
/// run row is gone.
pub async fn clear_extraction_state(
    pool: &sqlx::PgPool,
    doc_id: &str,
    post_clear: PostClearStatus,
) -> Result<(), AppError> {
    let mut txn = pool.begin().await.map_err(|e| AppError::Internal {
        message: format!("Failed to begin extraction-clear transaction for '{doc_id}': {e}"),
    })?;

    // ORDER IS FK-SAFE AND IS NOT COSMETIC — see `CLEAR_STATEMENTS`.
    for sql in CLEAR_STATEMENTS {
        sqlx::query(sql)
            .bind(doc_id)
            .execute(&mut *txn)
            .await
            .map_err(|e| AppError::Internal {
                message: format!(
                    "Clearing extraction state for '{doc_id}' failed on `{sql}`: {e}. \
                     The transaction is rolled back, so the document is unchanged."
                ),
            })?;
    }

    // The two preserved step names are bound as parameters rather than inlined
    // so they stay tied to the canonical `STEP_*` constants (Rule 2) and cannot
    // drift from the names `record_step_start` actually writes.
    sqlx::query(
        "DELETE FROM pipeline_steps \
         WHERE document_id = $1 AND step_name NOT IN ($2, $3)",
    )
    .bind(doc_id)
    .bind(STEP_UPLOAD)
    .bind(STEP_EXTRACT_TEXT)
    .execute(&mut *txn)
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Delete pipeline_steps for '{doc_id}': {e}"),
    })?;

    if post_clear == PostClearStatus::ResetToTextExtracted {
        sqlx::query("UPDATE documents SET status = $1, updated_at = NOW() WHERE id = $2")
            .bind(STATUS_TEXT_EXTRACTED)
            .bind(doc_id)
            .execute(&mut *txn)
            .await
            .map_err(|e| AppError::Internal {
                message: format!(
                    "Update documents.status → {STATUS_TEXT_EXTRACTED} for '{doc_id}': {e}. \
                     The clearing transaction is rolled back, so the document is \
                     unchanged. Check PostgreSQL connectivity before retrying."
                ),
            })?;
    }

    txn.commit().await.map_err(|e| AppError::Internal {
        message: format!("Failed to commit extraction-clear transaction for '{doc_id}': {e}"),
    })?;

    tracing::info!(
        doc_id = %doc_id,
        post_clear = ?post_clear,
        "Extraction state cleared — the next run will re-run both LLM passes"
    );
    Ok(())
}

// ── Reprocess ──────────────────────────────────────────────────

/// Widened relationships-DELETE for the reprocess path (DELETE-FK-FIX).
///
/// Identical in intent to
/// `documents_delete::DELETE_RELATIONSHIPS_TOUCHING_DOCUMENT` (kept as a
/// separate copy by design — the three delete paths are fixed in place, not
/// refactored into one shared helper). Matches every relationship that touches
/// this document: rows it owns (`document_id`) AND rows another document owns
/// that point at this document's items via either RESTRICT FK (`from_item_id` /
/// `to_item_id`). Without the endpoint predicates a foreign relationship
/// targeting this document's items survives and trips the FK on the
/// `extraction_items` delete, rolling the reprocess back.
const REPROCESS_DELETE_RELATIONSHIPS_SQL: &str = "DELETE FROM extraction_relationships \
     WHERE document_id = $1 \
        OR from_item_id IN (SELECT id FROM extraction_items WHERE document_id = $1) \
        OR to_item_id IN (SELECT id FROM extraction_items WHERE document_id = $1)";

/// POST /documents/:id/reprocess — full reset to TEXT_EXTRACTED for re-extraction.
///
/// Cleans Neo4j + Qdrant (best-effort), deletes extraction data in FK-safe
/// order inside a PG transaction, then resets document status to
/// TEXT_EXTRACTED so "Analyze Content" becomes available again.
pub async fn reprocess_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<Json<ReprocessResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST reprocess");

    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to fetch document '{doc_id}' for reprocess: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    if !matches!(
        document.status.as_str(),
        STATUS_INGESTED | STATUS_INDEXED | STATUS_PUBLISHED
    ) {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot reprocess: status is '{}', expected {STATUS_INGESTED}, {STATUS_INDEXED}, or {STATUS_PUBLISHED}",
                document.status
            ),
            details: serde_json::json!({"status": document.status}),
        });
    }

    // Cross-store cleanup is deliberately deferred until AFTER the Postgres
    // transaction commits — see the post-commit block below (DELETE-ORDER-FIX).
    clear_extraction_state(
        &state.pipeline_pool,
        &doc_id,
        PostClearStatus::ResetToTextExtracted,
    )
    .await?;

    // Cross-store cleanup AFTER the Postgres commit (DELETE-ORDER-FIX).
    //
    // Why: Postgres is the source of truth. These deletes used to run BEFORE
    // the transaction, so a failing/rolled-back PG reprocess left the document
    // half-wiped — purged from Neo4j + Qdrant but still fully present in
    // Postgres. A PRE-commit cross-store wipe is destructive and unrecoverable;
    // a POST-commit failure is recoverable (best-effort, logged inside each
    // helper). Run them only once the extraction tier is durably cleared. Do
    // not reorder these back above the transaction.
    cleanup_neo4j(&state, &doc_id).await;
    cleanup_qdrant(&state, &doc_id).await;

    log_admin_action(
        &state.audit_repo,
        &user.username,
        "pipeline.document.reprocess",
        Some("document"),
        Some(&doc_id),
        Some(serde_json::json!({"previous_status": document.status})),
    )
    .await;

    tracing::info!(
        doc_id = %doc_id, previous = %document.status,
        "Document reprocessed — status → TEXT_EXTRACTED"
    );

    Ok(Json(ReprocessResponse {
        document_id: doc_id,
        status: STATUS_TEXT_EXTRACTED.to_string(),
        message: "Document reset for re-extraction. Select schema and run Analyze Content."
            .to_string(),
    }))
}

// ── Bulk Approve ────────────────────────────────────────────────

/// POST /documents/:id/approve-all
pub async fn bulk_approve_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
    Json(body): Json<BulkApproveRequest>,
) -> Result<Json<BulkApproveResponse>, AppError> {
    require_admin(&user)?;

    if body.filter != "grounded" && body.filter != "all" {
        return Err(AppError::BadRequest {
            message: format!(
                "Invalid filter '{}' — must be 'grounded' or 'all'",
                body.filter
            ),
            details: serde_json::json!({"field": "filter", "valid": ["grounded", "all"]}),
        });
    }

    let approved_count =
        review_repo::bulk_approve(&state.pipeline_pool, &doc_id, &user.username, &body.filter)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Bulk approve failed: {e}"),
            })?;

    let remaining_pending = review_repo::count_pending(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Count pending failed: {e}"),
        })?;

    let skipped_ungrounded = review_repo::count_ungrounded_pending(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Count ungrounded failed: {e}"),
        })?;

    if let Ok(sid) = steps::record_step_start(
        &state.pipeline_pool,
        &doc_id,
        "bulk_approve",
        &user.username,
        &serde_json::json!({"filter": body.filter}),
    )
    .await
    {
        if let Err(e) = steps::record_step_complete(
            &state.pipeline_pool,
            sid,
            0.0,
            &serde_json::json!({
                "approved_count": approved_count,
                "skipped_ungrounded": skipped_ungrounded,
                "remaining_pending": remaining_pending,
            }),
        )
        .await
        {
            tracing::error!(
                document_id = %doc_id,
                step_id = sid,
                error = %e,
                "Failed to record bulk_approve step completion — audit trail gap"
            );
        }
    }

    Ok(Json(BulkApproveResponse {
        document_id: doc_id,
        approved_count,
        skipped_ungrounded,
        remaining_pending,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The clear must delete children before parents, or a document with review
    /// history rolls the whole transaction back on a RESTRICT foreign key.
    ///
    /// Asserted as an ORDER rather than as a set: the statements are all valid
    /// SQL in any order, and only the sequence is wrong. This is the half of
    /// `clear_extraction_state` that can be checked without a live database; the
    /// other half — that the right rows actually go — is proved on DEV by P1's
    /// new `extraction_runs` row id.
    #[test]
    fn clear_statements_delete_children_before_parents() {
        let position = |needle: &str| {
            CLEAR_STATEMENTS
                .iter()
                .position(|s| s.contains(needle))
                .unwrap_or_else(|| panic!("no clear statement mentions {needle}"))
        };

        let history = position("review_edit_history");
        let relationships = position("extraction_relationships");
        let items = position("DELETE FROM extraction_items");
        let runs = position("DELETE FROM extraction_runs");

        assert!(
            history < items,
            "review_edit_history references extraction_items — it must go first",
        );
        assert!(
            relationships < items,
            "extraction_relationships references extraction_items on both endpoints",
        );
        assert!(
            items < runs,
            "extraction_items references extraction_runs — items must go first",
        );
        assert_eq!(
            CLEAR_STATEMENTS.len(),
            4,
            "a statement was added or removed without revisiting the FK order",
        );
    }

    /// `pipeline_steps` is cleared separately because it binds the two step
    /// names it preserves. If it ever migrates into `CLEAR_STATEMENTS` it will
    /// silently lose that filter and wipe the upload and text-extraction rows,
    /// which would make every re-extraction re-OCR the PDF.
    #[test]
    fn the_bulk_clear_does_not_touch_pipeline_steps_or_documents() {
        for sql in CLEAR_STATEMENTS {
            assert!(
                !sql.contains("pipeline_steps"),
                "pipeline_steps needs its step-name filter: {sql}",
            );
            assert!(
                !sql.contains("UPDATE documents") && !sql.contains("DELETE FROM documents"),
                "the clear must never touch the documents row itself: {sql}",
            );
        }
    }

    /// DELETE-FK-FIX guard for the reprocess path: the relationships clear must
    /// match BOTH item-endpoint FKs, not just the owning `document_id`. Without
    /// this, reprocessing a document whose items are targeted by another
    /// document's relationship rolls back on the RESTRICT FK. There is no
    /// `#[sqlx::test]` / live-DB harness in this repo, so the widening is
    /// verified by asserting the SQL covers both endpoints; the end-to-end
    /// behaviour is verified manually on DEV.
    #[test]
    fn reprocess_delete_relationships_sql_covers_both_fk_endpoints() {
        let sql = REPROCESS_DELETE_RELATIONSHIPS_SQL;
        assert!(
            sql.contains("document_id = $1"),
            "must still clear rows this document owns"
        );
        assert!(
            sql.contains(
                "from_item_id IN (SELECT id FROM extraction_items WHERE document_id = $1)"
            ),
            "must clear rows pointing FROM this document's items"
        );
        assert!(
            sql.contains("to_item_id IN (SELECT id FROM extraction_items WHERE document_id = $1)"),
            "must clear rows pointing TO this document's items (the RESTRICT endpoint)"
        );
    }
}
