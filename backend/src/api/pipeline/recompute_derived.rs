//! POST /api/admin/pipeline/documents/:id/recompute-derived-grounding
//!
//! Re-runs v5.1 §5.4 derived-provenance validation over a document's
//! existing extraction items WITHOUT re-running verbatim text matching,
//! re-extracting, or transitioning the document's lifecycle status.
//! Use case: a document was verified before v5.1 landed (so all derived
//! items got the blanket `derived` stamp) and Roman wants to backfill
//! the validated `derived` / `derived_invalid` distinction without a
//! full pipeline re-run.
//!
//! ## Rust Learning: a thin "diagnostic re-run" endpoint
//!
//! This endpoint is deliberately narrow — three jobs:
//!   1. Build the same `paragraph_number → item_id` map the verifier
//!      builds during a normal verify run.
//!   2. Re-classify every derived-mode item via the shared pure
//!      `validate_derived_provenance` helper.
//!   3. Write the new `(grounding_status, verification_reason)` pair
//!      back to PostgreSQL.
//!
//! It does NOT write to Neo4j, does NOT touch non-derived items, and
//! does NOT change `documents.status`. Idempotent: running it twice on
//! unchanged data produces the same result.
//!
//! ## Status guard
//!
//! Pre-verify states (`UPLOADED` / `EXTRACTED`) are rejected with 409
//! because their `extraction_items` rows have no meaningful
//! `grounding_status` to recompute. Post-verify states all proceed.

use std::collections::HashMap;

use axum::{extract::Path as AxumPath, extract::State, Json};
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::models::document_status::{
    STATUS_COMPLETED, STATUS_INDEXED, STATUS_INGESTED, STATUS_PUBLISHED, STATUS_VERIFIED,
};
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository;
use crate::state::AppState;

use super::verify::{
    build_para_to_item_id, load_grounding_config, validate_derived_provenance, DerivedValidation,
};

/// Response body for the recompute endpoint.
///
/// `reasons` is a histogram of `verification_reason` strings → count.
/// Roman uses it to confirm the May-5 Awad doc tally without psql:
/// the predicted body is `{ "no provenance array — schema/template
/// gap …": 13, "item_data is null": 1 }` with `marked_valid: 7`,
/// `marked_invalid: 14`.
#[derive(Debug, Serialize)]
pub struct RecomputeDerivedResponse {
    pub document_id: String,
    pub total_derived_items: usize,
    pub marked_valid: usize,
    pub marked_invalid: usize,
    pub reasons: HashMap<String, usize>,
    pub duration_secs: f64,
}

/// POST /api/admin/pipeline/documents/:id/recompute-derived-grounding
///
/// Admin-only. Status guard: document must be post-verify.
pub async fn recompute_derived_grounding_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<Json<RecomputeDerivedResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST recompute-derived-grounding");

    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    // Only documents whose extraction_items rows already have meaningful
    // grounding_status values can be recomputed. Pre-verify states have
    // NULL grounding_status everywhere — recomputing would be a no-op
    // at best and confusing at worst.
    if !matches!(
        document.status.as_str(),
        STATUS_VERIFIED | STATUS_INGESTED | STATUS_INDEXED | STATUS_PUBLISHED | STATUS_COMPLETED
    ) {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot recompute derived grounding: status is '{}', expected post-verify state \
                 ({STATUS_VERIFIED} | {STATUS_INGESTED} | {STATUS_INDEXED} | {STATUS_PUBLISHED} | {STATUS_COMPLETED})",
                document.status
            ),
            details: serde_json::json!({ "status": document.status }),
        });
    }

    let result = run_recompute_derived(&state, &doc_id, &user.username).await?;
    Ok(Json(result))
}

/// Core fn — does the actual work. Separated from the handler so other
/// callers (e.g., a future bulk endpoint or pipeline-step) can re-use
/// it without going through axum extraction.
pub(crate) async fn run_recompute_derived(
    state: &AppState,
    doc_id: &str,
    username: &str,
) -> Result<RecomputeDerivedResponse, AppError> {
    let start = std::time::Instant::now();

    // 1. Load the schema's verification config — same call the regular
    //    verify path uses. This is a Result-returning fn, so the failure
    //    mode is loud, not silent (matches CLAUDE.md Rule 1).
    let grounding_config = load_grounding_config(
        &state.pipeline_pool,
        &state.config.extraction_schema_dir,
        doc_id,
    )
    .await
    .map_err(|e| {
        tracing::error!(
            document_id = %doc_id, error = %e,
            "Recompute cannot proceed without grounding config"
        );
        AppError::Internal { message: e }
    })?;

    // 2. Pull every extraction_item for this document. We need ALL items
    //    (not just derived) because `build_para_to_item_id` indexes the
    //    Allegations to resolve provenance refs against.
    let items = pipeline_repository::get_all_items(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?;

    let para_to_item_id = build_para_to_item_id(&items);

    // 3. Re-validate every derived-mode item. Counting and reason-histogram
    //    happen inline; the histogram surfaces in the response so Roman
    //    can confirm the failure shape without psql.
    let mut total_derived_items = 0usize;
    let mut marked_valid = 0usize;
    let mut marked_invalid = 0usize;
    let mut reasons: HashMap<String, usize> = HashMap::new();

    for item in &items {
        // Skip non-derived items entirely. The endpoint's contract is
        // narrow: only derived items are touched. Verbatim/name/heading/
        // none modes are not re-evaluated here.
        let cfg = match grounding_config.get(&item.entity_type) {
            Some(c) if matches!(c.mode, colossus_extract::GroundingMode::Derived) => c,
            _ => continue,
        };

        total_derived_items += 1;

        let validation =
            validate_derived_provenance(item, &para_to_item_id, cfg.provenance_required);
        let (status_str, reason) = match validation {
            DerivedValidation::Valid => {
                marked_valid += 1;
                ("derived", None)
            }
            DerivedValidation::Invalid(r) => {
                marked_invalid += 1;
                *reasons.entry(r.clone()).or_insert(0) += 1;
                ("derived_invalid", Some(r))
            }
        };

        pipeline_repository::update_item_grounding(
            &state.pipeline_pool,
            item.id,
            status_str,
            None,
            reason.as_deref(),
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update item {}: {e}", item.id),
        })?;
    }

    let duration = start.elapsed().as_secs_f64();

    tracing::info!(
        doc_id = %doc_id,
        total_derived_items,
        marked_valid,
        marked_invalid,
        duration_secs = format!("{duration:.2}"),
        "Recompute derived grounding complete"
    );

    log_admin_action(
        &state.audit_repo,
        username,
        "pipeline.document.recompute_derived_grounding",
        Some("document"),
        Some(doc_id),
        Some(serde_json::json!({
            "total_derived_items": total_derived_items,
            "marked_valid": marked_valid,
            "marked_invalid": marked_invalid,
            "reasons": &reasons,
        })),
    )
    .await;

    Ok(RecomputeDerivedResponse {
        document_id: doc_id.to_string(),
        total_derived_items,
        marked_valid,
        marked_invalid,
        reasons,
        duration_secs: duration,
    })
}
