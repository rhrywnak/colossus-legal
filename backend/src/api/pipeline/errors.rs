//! Error tracking endpoint — failed pipeline steps, and the attention count.
//!
//! ## Two different questions, which used to be one answer
//!
//! `documents_with_errors` answers "which steps have ever failed?" — a
//! diagnostic history, and a step that failed once and succeeded on retry
//! belongs in it forever, because it did fail.
//!
//! `needs_attention` answers "how many documents need a human RIGHT NOW?" — and
//! that is a property of the document's CURRENT status, not of its history.
//!
//! The Documents page banner asked the first question and displayed the answer
//! as though it were the second. Measured on DEV 2026-08-17:
//! `doc-george-phillips-admissions-response` had `llm_extract_pass2` fail at
//! 18:41:42 ("Retryable error … Will retry") and COMPLETE at 18:44:21; every
//! other step completed; the document is PUBLISHED with 156/156 grounded. The
//! banner counted it, and would have counted it forever, because the failed step
//! row never goes away. A retry that succeeded is the system working, not a
//! document needing attention.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::document_status::{STATUS_FAILED, STATUS_IN_REVIEW};
use crate::state::AppState;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DocumentError {
    pub document_id: String,
    pub document_title: String,
    pub document_status: String,
    pub failed_step: String,
    pub error_message: Option<String>,
    pub failed_at: String,
    pub triggered_by: Option<String>,
    pub retry_count: i64,
}

#[derive(Debug, Serialize)]
pub struct ErrorsResponse {
    /// Every step that has ever failed, most recent first. A diagnostic history:
    /// a step that failed and then succeeded on retry stays here.
    pub documents_with_errors: Vec<DocumentError>,
    /// How many documents appear in that history. **Not** the banner's number —
    /// see [`ErrorsResponse::needs_attention`].
    pub total_errors: i64,
    pub documents_with_no_errors: i64,
    /// Documents whose CURRENT status needs a human: failed, or awaiting review.
    ///
    /// This is what the Documents page banner counts. A retried-then-completed
    /// step never reaches it, because it is computed from `documents.status` and
    /// not from step history at all.
    pub needs_attention: i64,
}

/// GET /documents/errors — returns all documents with failed pipeline steps.
pub async fn errors_handler(
    _user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ErrorsResponse>, AppError> {
    let pool = &state.pipeline_pool;

    // Get documents with their most recent failed step
    let errors: Vec<DocumentError> = sqlx::query_as::<_, DocumentError>(
        r#"SELECT
            d.id AS document_id,
            d.title AS document_title,
            d.status AS document_status,
            ps.step_name AS failed_step,
            ps.error_message,
            ps.started_at::text AS failed_at,
            ps.triggered_by,
            (SELECT COUNT(*) FROM pipeline_steps ps2
             WHERE ps2.document_id = d.id AND ps2.step_name = ps.step_name
            ) AS retry_count
        FROM documents d
        JOIN pipeline_steps ps ON ps.document_id = d.id
        WHERE ps.status = 'failed'
          AND ps.id = (
              SELECT ps3.id FROM pipeline_steps ps3
              WHERE ps3.document_id = d.id AND ps3.status = 'failed'
              ORDER BY ps3.started_at DESC
              LIMIT 1
          )
        ORDER BY ps.started_at DESC"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to query errors: {e}"),
    })?;

    let total_errors = errors.len() as i64;

    // The attention count, from CURRENT status only. Bound as an array rather
    // than interpolated so the two statuses stay tied to the `STATUS_*`
    // constants (Rule 2) and cannot drift from the vocabulary the rest of the
    // pipeline writes.
    //
    // Domain note on the two members: FAILED is a document the pipeline could
    // not finish, and IN_REVIEW is one waiting on a person. Both need a human.
    // CANCELLED is deliberately absent — someone already decided that one.
    let attention_statuses = vec![STATUS_FAILED.to_string(), STATUS_IN_REVIEW.to_string()];
    let needs_attention: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM documents WHERE status = ANY($1)")
            .bind(&attention_statuses)
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Internal {
                message: format!(
                    "Failed to count documents needing attention: {e}. The Documents \
                     page banner cannot be rendered; check PIPELINE_DATABASE_URL."
                ),
            })?;

    let total_docs: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents")
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to count documents: {e}"),
        })?;

    Ok(Json(ErrorsResponse {
        documents_with_errors: errors,
        total_errors,
        documents_with_no_errors: total_docs.0 - total_errors,
        needs_attention: needs_attention.0,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The attention set is FAILED and IN_REVIEW — and nothing else.
    ///
    /// Pinned because the whole defect was a count that answered a different
    /// question: a document is "needing attention" by its CURRENT status, never
    /// by whether some step once failed. CANCELLED is deliberately out (someone
    /// already decided), and so is every mid-pipeline status (the pipeline is
    /// still working on it).
    #[test]
    fn the_attention_statuses_are_failed_and_in_review_only() {
        let attention = [STATUS_FAILED, STATUS_IN_REVIEW];
        assert_eq!(attention.len(), 2);
        assert!(attention.contains(&"FAILED"));
        assert!(attention.contains(&"IN_REVIEW"));

        for not_attention in [
            crate::models::document_status::STATUS_PUBLISHED,
            crate::models::document_status::STATUS_COMPLETED,
            crate::models::document_status::STATUS_CANCELLED,
            crate::models::document_status::STATUS_PROCESSING,
            crate::models::document_status::STATUS_EXTRACTED,
        ] {
            assert!(
                !attention.contains(&not_attention),
                "{not_attention} must not count as needing attention",
            );
        }
    }

    /// The response carries BOTH numbers under distinct names, so a future
    /// reader cannot wire the banner back to the step history by accident.
    #[test]
    fn the_response_separates_step_history_from_the_attention_count() {
        let json = serde_json::to_string(&ErrorsResponse {
            documents_with_errors: vec![],
            total_errors: 1,
            documents_with_no_errors: 10,
            needs_attention: 0,
        })
        .expect("the response must serialize");

        // The live DEV shape after this fix: one document has a failed step in
        // its history, and zero documents need attention.
        assert!(json.contains(r#""total_errors":1"#), "{json}");
        assert!(json.contains(r#""needs_attention":0"#), "{json}");
    }
}
